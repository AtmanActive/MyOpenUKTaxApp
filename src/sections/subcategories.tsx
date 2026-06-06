// Section 4 - Subcategory (Category) Management.
//
// Users create and rename their own categories under the two built-in kinds
// (Income / Expenses). A category that is already used by an event cannot be
// deleted, only renamed — enforced by the backend and reflected in the UI.

import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
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
import type { Kind, Subcategory } from "@/lib/types";
import { notify_error, use_notify } from "@/store/notify";

export function SubcategoriesSection()
{
	const push = use_notify((state) => state.push);
	const query_client = useQueryClient();

	const [new_kind, set_new_kind] = useState<Kind>("expense");
	const [new_name, set_new_name] = useState("");
	const [new_description, set_new_description] = useState("");
	const [editing_id, set_editing_id] = useState<number | null>(null);
	const [edit_name, set_edit_name] = useState("");
	const [edit_description, set_edit_description] = useState("");

	const subcategories_query = useQuery({
		queryKey: ["subcategories"],
		queryFn: () => api.list_subcategories(),
	});

	// Renaming or deleting changes names that appear elsewhere, so invalidate
	// the dependent caches too.
	const refresh = () =>
	{
		void query_client.invalidateQueries({ queryKey: ["subcategories"] });
		void query_client.invalidateQueries({ queryKey: ["events"] });
		void query_client.invalidateQueries({ queryKey: ["dashboard"] });
		void query_client.invalidateQueries({ queryKey: ["mappings"] });
	};

	const create_mutation = useMutation({
		mutationFn: () =>
		{
			if (!new_name.trim())
			{
				return Promise.reject("Enter a category name.");
			}
			return api.create_subcategory(new_kind, new_name, new_description);
		},
		onSuccess: () =>
		{
			push("success", "Category created.");
			set_new_name("");
			set_new_description("");
			refresh();
		},
		onError: (error) => notify_error(error),
	});

	const update_mutation = useMutation({
		mutationFn: (id: number) => api.update_subcategory(id, edit_name, edit_description),
		onSuccess: () =>
		{
			push("success", "Category updated.");
			set_editing_id(null);
			refresh();
		},
		onError: (error) => notify_error(error),
	});

	const delete_mutation = useMutation({
		mutationFn: (id: number) => api.delete_subcategory(id),
		onSuccess: () =>
		{
			push("success", "Category deleted.");
			refresh();
		},
		onError: (error) => notify_error(error),
	});

	// Enter inline edit mode for a row.
	const start_edit = (item: Subcategory) =>
	{
		set_editing_id(item.id);
		set_edit_name(item.name);
		set_edit_description(item.description);
	};

	const rows = subcategories_query.data ?? [];

	return (
		<div className="flex flex-col gap-4">
			<Card>
				<CardHeader>
					<CardTitle>Add a category</CardTitle>
				</CardHeader>
				<CardContent>
					<div className="grid grid-cols-1 gap-3 sm:grid-cols-[10rem_1fr_1fr_auto] sm:items-end">
						<div className="flex flex-col gap-1.5">
							<Label htmlFor="new_kind">Type</Label>
							<Select
								id="new_kind"
								title="Whether this category is for income or expenses"
								value={new_kind}
								onChange={(event) => set_new_kind(event.target.value as Kind)}
							>
								<option value="income">Income</option>
								<option value="expense">Expense</option>
							</Select>
						</div>
						<div className="flex flex-col gap-1.5">
							<Label htmlFor="new_name">Name</Label>
							<Input
								id="new_name"
								title="The category name"
								placeholder="e.g. Software"
								value={new_name}
								onChange={(event) => set_new_name(event.target.value)}
							/>
						</div>
						<div className="flex flex-col gap-1.5">
							<Label htmlFor="new_description">Description</Label>
							<Input
								id="new_description"
								title="An optional description"
								placeholder="optional"
								value={new_description}
								onChange={(event) => set_new_description(event.target.value)}
							/>
						</div>
						<Button
							title="Create this category"
							disabled={create_mutation.isPending}
							onClick={() => create_mutation.mutate()}
						>
							<Icon name="add" /> Add
						</Button>
					</div>
				</CardContent>
			</Card>

			<Card>
				<CardHeader>
					<CardTitle>Your categories</CardTitle>
				</CardHeader>
				<CardContent>
					<Table>
						<TableHeader>
							<TableRow>
								<TableHead>Type</TableHead>
								<TableHead>Name</TableHead>
								<TableHead>Description</TableHead>
								<TableHead className="text-right">Actions</TableHead>
							</TableRow>
						</TableHeader>
						<TableBody>
							{rows.map((item) =>
							{
								const is_editing = editing_id === item.id;
								return (
									<TableRow key={item.id}>
										<TableCell>
											<span className={item.kind === "income" ? "text-income" : "text-expense"}>
												{item.kind}
											</span>
										</TableCell>
										<TableCell className="font-medium">
											{is_editing ? (
												<Input value={edit_name} onChange={(event) => set_edit_name(event.target.value)} />
											) : (
												item.name
											)}
										</TableCell>
										<TableCell className="text-muted-foreground">
											{is_editing ? (
												<Input
													value={edit_description}
													onChange={(event) => set_edit_description(event.target.value)}
												/>
											) : (
												item.description || "—"
											)}
										</TableCell>
										<TableCell className="text-right">
											<div className="flex justify-end gap-1">
												{is_editing ? (
													<>
														<Button
															variant="ghost"
															size="icon"
															title="Save changes"
															onClick={() => update_mutation.mutate(item.id)}
														>
															<Icon name="check" className="text-base text-income" />
														</Button>
														<Button
															variant="ghost"
															size="icon"
															title="Cancel"
															onClick={() => set_editing_id(null)}
														>
															<Icon name="close" className="text-base" />
														</Button>
													</>
												) : (
													<>
														<Button
															variant="ghost"
															size="icon"
															title="Rename / edit this category"
															onClick={() => start_edit(item)}
														>
															<Icon name="edit" className="text-base" />
														</Button>
														<Button
															variant="ghost"
															size="icon"
															title={
																item.in_use
																	? "In use by events — rename only, cannot delete"
																	: "Delete this category"
															}
															disabled={item.in_use}
															onClick={() =>
															{
																if (window.confirm(`Delete category "${item.name}"?`))
																{
																	delete_mutation.mutate(item.id);
																}
															}}
														>
															<Icon
																name="delete"
																className={item.in_use ? "text-base opacity-40" : "text-base text-destructive"}
															/>
														</Button>
													</>
												)}
											</div>
										</TableCell>
									</TableRow>
								);
							})}
						</TableBody>
					</Table>
				</CardContent>
			</Card>
		</div>
	);
}
