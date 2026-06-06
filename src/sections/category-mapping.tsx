// Section 5 - Category Mapping.
//
// Each user subcategory can map to a single HMRC category; many subcategories
// may share the same HMRC category (many-to-one). The mapping is later used to
// translate the user's categories into HMRC's when submitting quarterly data.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Spinner } from "@/components/ui/spinner";
import { Select } from "@/components/ui/select";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components/ui/table";
import { api } from "@/lib/api";
import { notify_error, use_notify } from "@/store/notify";

export function CategoryMappingSection()
{
	const push = use_notify((state) => state.push);
	const query_client = useQueryClient();

	const subcategories_query = useQuery({
		queryKey: ["subcategories"],
		queryFn: () => api.list_subcategories(),
	});
	const hmrc_categories_query = useQuery({
		queryKey: ["hmrc_categories"],
		queryFn: () => api.list_hmrc_categories(),
	});
	const mappings_query = useQuery({
		queryKey: ["mappings"],
		queryFn: () => api.list_category_mappings(),
	});

	const refresh = () => void query_client.invalidateQueries({ queryKey: ["mappings"] });

	const set_mutation = useMutation({
		mutationFn: (vars: { subcategory_id: number; hmrc_category_id: number }) =>
			api.set_category_mapping(vars),
		onSuccess: () =>
		{
			push("success", "Mapping saved.");
			refresh();
		},
		onError: (error) => notify_error(error),
	});

	const clear_mutation = useMutation({
		mutationFn: (mapping_id: number) => api.delete_category_mapping(mapping_id),
		onSuccess: () =>
		{
			push("success", "Mapping cleared.");
			refresh();
		},
		onError: (error) => notify_error(error),
	});

	if (subcategories_query.isLoading || hmrc_categories_query.isLoading || mappings_query.isLoading)
	{
		return <Spinner label="Loading mappings…" />;
	}

	const subcategories = subcategories_query.data ?? [];
	const hmrc_categories = hmrc_categories_query.data ?? [];
	const mappings = mappings_query.data ?? [];

	// Look up the current mapping for a subcategory, if any.
	const mapping_for = (subcategory_id: number) =>
		mappings.find((mapping) => mapping.subcategory_id === subcategory_id);

	return (
		<Card>
			<CardHeader>
				<CardTitle>Map your categories to HMRC categories</CardTitle>
			</CardHeader>
			<CardContent>
				<Table>
					<TableHeader>
						<TableRow>
							<TableHead>Your category</TableHead>
							<TableHead>Type</TableHead>
							<TableHead>HMRC category</TableHead>
						</TableRow>
					</TableHeader>
					<TableBody>
						{subcategories.map((subcategory) =>
						{
							const current = mapping_for(subcategory.id);
							// Only HMRC categories of the same kind are valid targets.
							const options = hmrc_categories.filter((item) => item.kind === subcategory.kind);
							return (
								<TableRow key={subcategory.id}>
									<TableCell className="font-medium">{subcategory.name}</TableCell>
									<TableCell>
										<span className={subcategory.kind === "income" ? "text-income" : "text-expense"}>
											{subcategory.kind}
										</span>
									</TableCell>
									<TableCell>
										<Select
											className="max-w-md"
											title={`Choose the HMRC category for "${subcategory.name}"`}
											value={current?.hmrc_category_id ?? ""}
											onChange={(event) =>
											{
												const value = event.target.value;
												if (value === "")
												{
													if (current)
													{
														clear_mutation.mutate(current.id);
													}
												}
												else
												{
													set_mutation.mutate({
														subcategory_id: subcategory.id,
														hmrc_category_id: Number(value),
													});
												}
											}}
										>
											<option value="">— not mapped —</option>
											{options.map((item) => (
												<option key={item.id} value={item.id}>
													{item.label} ({item.code})
												</option>
											))}
										</Select>
									</TableCell>
								</TableRow>
							);
						})}
					</TableBody>
				</Table>
			</CardContent>
		</Card>
	);
}
