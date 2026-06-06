# The MyOpenUKTaxApp project

The goal of this project is to build a multi-platform desktop app for working with UK tax accounting.
The app should work like a portable desktop app, with the online API client capability to submit data directly to [HMRC MTD](https://www.gov.uk/government/collections/making-tax-digital-for-income-tax) using their [HTTPS API](https://developer.service.hmrc.gov.uk/api-documentation/docs/api).

As a simple accounting app, the app will hold the database of two major categories: Income and Expenses. Users will be able to use a nice form to input income events and expense events. The app will then be able to show tables of data from and to any user-defined period of time together with sums. The users will be able to define subcategories independently for income and expenses and will be able to assign the subcategory on event input and filter display by those subcategories on table output. HMRC will have their own categories which will be retrieved via API and stored in a separate table which will always be read-only from user's perspective. There will be one more user-editable table called "Category Mapping" where users will be able to map their own subcategories to HMRC categories. This mapping will then be used when submitting data to HMRC MTD API quaterly.

### Path portability
The app should never assume any well known directories on the underlying operating system. Instead, the app should self-discover the exe's path and use that to infer all other needed paths in a relative way. The app and all of it's data must exist inside just one directory. Under no circumstances should app source/binary/data files be scattered acros multiple directories. The app is allowed to create and use an unlimited number of subdirectories that are positioned inside it's main directory.

### Settings
The app will be saving it's settings into an exe-adjacent JSON file. If `MyOpenUKTaxApp.settings.json` file doesn't exist, a new one should be created.

### Data
The app will be saving all user data as an SQLite file `Data/MyOpenUKTaxApp.db`. If `Data` subdirectory doesn't exist, it should be created by the app. If the SQLite file doesn't exist, it should be created by the app. The app should maintain it's own step database backup(s) in the `Data/Backups` subdirectory: each time the main database file is about to be changed, a copy of the SQLite file should be created with the file name corresponding to current date and time. Files older than X days should be automatically deleted. This DB backup-and-pruning routine should be activated automatically but in a smart way so that batch DB operations do not create hundreds of unnecessary backups. The default value for backups-pruned-after-days should be 1200.

### Logs
The app will be saving it's logs to folder `Logs`. If `Logs` subdirectory doesn't exist, it should be created by the app. In the directory `Logs` there will be three subdirectories: `Action`, `Debug` and `Network`, if any of these subdirectories is missing, it will be automatically created by the app. Logs will be continually written to these three subdirectories. The log files will be named starting with date and time.miliseconds converted to filename-acceptable format, like so: YYYY-MM-DD_HH-MM-SS-MS_MyOpenUKTaxApp_Action.log. Action logs will only record what user clicked. Debug logs will log detailed information about everything, including line numbers and file names of code responsible. Network logs will record only HTTP(S) requests and responses. The app should automatically prune it's log files (delete files older than X), according to user preferences. The default value for logs-pruned-after-days should be 2200.

### User Interface
The app will be running as a normal desktop app with it's own window. No need for tray icon or persistence. The app user interface needs to be fully adjustable in terms of text size and colours. The interface should support colour themes. There will be both light and dark themes. The app should automatically detect operating system's light/dark mode and activate the appropriate theme. The UI has to be responsive: the app elements should adjust and reorganize themselves depending on the available screen estate. The app will allow users to increase/decrease font size and will store this as a setting in it's JSON file and automatically apply on next run. The app should support mouse clicking actions and keyboard shortcuts. Each and every UI element should have a hint (HTML attribute "title") to display a few words of explanation when mouse hovers.

### User Interface Sections
#### Sidebar
There will be a vertical sidebar that is always present and will hold icons for main app menu.
#### Topbar
There will be a horizontal topbar that is always present and will hold search and pagination toolbar.
#### Main Pane
There will be a main pane to the right of the sidebar, and below the topbar, that will hold the contents the user is currently working on. The Main Pane should occupy all available remaining app window/screen space.
In case of vertical screen orientation (where window width is less than window height), the Sidebar should be presented as a horizontal taskbar, ocuppying the bottom of the screen.

### User Interface Widgets
User interface will need to use the following UI widgets: scrolling areas both vertically and horizontally, checkboxes, drop-down menus, tables, editable tables, number spinners, material icons, flashing text, text input fields, number input fields, date input fields.

### App Sections
- [1] Dashboard
- [2] Add Event
- [3] show recorded events (with filters)
- [4] Subcategory Management
- [5] Category Mapping
- [6] HMRC post history
- [7] Settings
Sidebar menu should hold icons for all of these screens plus one more Exit icon at the bottom.

Topbar contents will dynamically change depending on what app section is currently displayed. Most of the time it will hold a search affordance and pagination affordances, but not always, depending on context. For example, there will be no search affordance on the Dashboard.

#### App Section 1 - Dashboard
Dashboard is the default view. It should hold all of the statistics and sums. All numbers and widgets should be clickable for further inspection or for jumping to another app section. We will flesh out the details as we go along.

#### App Section 2 - Add Event
This is the main page for data entry. It should look like a form to allow users to input events. Each and every item must record: automatic date and time of the entry itself (not visible), date as set by the user (visible with calendar selection widget), main category choice (Income or Expenses) presented as a tab-switch at the top which, when flipped, changes the underlying fields and colors, subcategory choice in a form of a drop-down, and a GBP input box. This form screen needs to be able to also show pre-populated data, when looked up from elsewhere, in which case it would need to be read-only but with an added action button at the bottom labeled "Clone" which would, when clicked, flip the form from reading-the-existing-record mode to entering-a-new-record-mode but with pre-populated data from the clone source. We will flesh out the details as we go along.

#### App Section 3 - show recorded events (with filters)
This page should look like two horizontal panes each holding a table of records, Income on top, Expenses on bottom half. The tables need to be easily sortable just by clicking on their headers. There will be no pagination. Instead, there will be filters in the topbar that would allow users to search by term and to set starting and ending dates. We will flesh out the details as we go along.

#### App Section 4 - Subcategory Management
This screen is where users manage their categories. From user perspective these are categories, from app perspective these are subcategories as the app already has two main categories: Income and Expenses. Categories can be anything. Each category can hold the name and the description. Once a category is created and assigned to at least one ledger event, it can't be deleted anymore. Instead, it can be renamed at will. There should be several categories included by default: Income/Main, Expenses/Phone, Expenses/Internet, Expenses/Utilities, Expenses/Bank, Expenses/Capital. We will flesh out the details as we go along.

#### App Section 5 - Category Mapping
This screen is where users manage their own category mapping(s) to HMRC's categories. There has to be many-to-one mapping ability so users can map one or more of their categories to a single HMRC category. So, the app will internally record all events with user-set categories, but will submit quaterly data to HMRC using HMRC's categories, mapping and translating the categories on the fly. We will flesh out the details as we go along.

#### App Section 6 - HMRC post history
This is the screen where users control and inspect the HMRC MTD part. The page will show a table of events (sums) sent to HMRC quaterly. We will flesh out the details as we go along.

#### App Section 7 - Settings
This is the screen where users can set global application settings. There should a color theme drop-down choice of "System", "Light" and "Dark". Default is "System". Next, there should be font size choice, as a drop-down: "xxx-small", "xx-small", "x-small", "small", "medium", "large", "x-large", "xx-large" and "xxx-large". Default is "medium". We will flesh out the details as we go along.

### MCP Server
While the app is running, it should expose an MCP server to allow any AI LLM agent to control and query the application. We will flesh out the details as we go along.
