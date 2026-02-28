//! Structure overview output formatting.

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::section::{HeadingLevel, Section};
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};

pub fn print_structure() {
    let title = Prose::new("<b><u>Sniff Data Structure</u></b>").render(None);
    println!("\n{}\n", title);

    let intro = Prose::new(
        "Structural map of the data sniff can produce. Leaf fields and values are omitted.",
    )
    .render(None);
    println!("{}\n", intro);

    let mut root_table = Table::new()
        .with_columns(vec![
            TableColumn::new("Root"),
            TableColumn::new("Produced By"),
            TableColumn::new("Top-Level Sections"),
        ])
        .prefer_cursor_alignment();

    root_table.add_row(vec![
        TableCellContent::Text(Prose::new("<b>SniffResult</b>").render(None)),
        TableCellContent::Text("detect(), detect_with_config()".to_string()),
        TableCellContent::Text("os, hardware, network, filesystem".to_string()),
    ]);
    root_table.add_row(vec![
        TableCellContent::Text(Prose::new("<b>ProgramsInfo</b>").render(None)),
        TableCellContent::Text("ProgramsInfo::detect()".to_string()),
        TableCellContent::Text(
            "editors, utilities, language package managers, os package managers, tts clients, terminal apps, headless audio, ai clients"
                .to_string(),
        ),
    ]);
    root_table.add_row(vec![
        TableCellContent::Text(Prose::new("<b>ServicesInfo</b>").render(None)),
        TableCellContent::Text("detect_services()".to_string()),
        TableCellContent::Text("init system, host os, evidence, services".to_string()),
    ]);

    println!("{}\n", root_table.render(None));

    let sniff_section = build_sniff_result_section();
    println!("{}\n", sniff_section.render(None));

    let programs_section = build_programs_section();
    println!("{}\n", programs_section.render(None));

    let services_section = build_services_section();
    println!("{}\n", services_section.render(None));
}

fn build_sniff_result_section() -> Section {
    let mut section = Section::new(HeadingLevel::h2, "SniffResult");
    section.push(Prose::new(
        "<dim>System + repository detection results.</dim>",
    ));

    let mut root_list = UnorderedList::empty();

    let os_list = list_from(&[
        "identity + versions",
        "distribution details",
        "kernel, hostname, uptime",
        "system package managers",
        "locale settings",
        "timezone + NTP status",
    ]);
    add_node(&mut root_list, "<b>os</b>", os_list);

    let hardware_list = list_from(&[
        "cpu profile",
        "memory stats",
        "storage volumes",
        "gpu devices + capabilities",
        "audio devices + sample rates",
    ]);
    add_node(&mut root_list, "<b>hardware</b>", hardware_list);

    let network_list = list_from(&[
        "interfaces (flags + addresses)",
        "primary interface selection",
        "aggregated IP addresses",
        "permission state",
    ]);
    add_node(&mut root_list, "<b>network</b>", network_list);

    let mut filesystem_list = UnorderedList::empty();
    add_node(
        &mut filesystem_list,
        "<b>languages</b>",
        list_from(&[
            "primary language",
            "per-language breakdown",
            "total files analyzed",
        ]),
    );

    add_node(
        &mut filesystem_list,
        "<b>git</b>",
        list_from(&[
            "repo metadata (root, branches)",
            "status + file changes",
            "recent commits",
            "remotes + tracking",
            "worktrees",
            "git config",
        ]),
    );

    add_node(
        &mut filesystem_list,
        "<b>repo</b>",
        list_from(&[
            "monorepo detection",
            "package locations",
            "dependencies by kind",
        ]),
    );

    add_node(
        &mut filesystem_list,
        "<b>formatting</b>",
        list_from(&["editorconfig config", "sections + formatting rules"]),
    );

    add_node(&mut root_list, "<b>filesystem</b>", filesystem_list);

    section.push(root_list);
    section
}

fn build_programs_section() -> Section {
    let mut section = Section::new(HeadingLevel::h2, "ProgramsInfo");
    section.push(Prose::new(
        "<dim>Installed program inventories grouped by category.</dim>",
    ));

    let mut list = UnorderedList::empty();
    list.add(Prose::new("<b>editors</b>"));
    list.add(Prose::new("<b>utilities</b>"));
    list.add(Prose::new("<b>language package managers</b>"));
    list.add(Prose::new("<b>os package managers</b>"));
    list.add(Prose::new("<b>tts clients</b>"));
    list.add(Prose::new("<b>terminal apps</b>"));
    list.add(Prose::new("<b>headless audio</b>"));
    list.add(Prose::new("<b>ai cli tools</b>"));

    section.push(list);
    section.push(Prose::new(
        "<dim>Each category includes detection metadata like paths, versions, sources, and descriptions.</dim>",
    ));

    section
}

fn build_services_section() -> Section {
    let mut section = Section::new(HeadingLevel::h2, "ServicesInfo");
    section.push(Prose::new(
        "<dim>Init system detection plus a service inventory.</dim>",
    ));

    let mut list = UnorderedList::empty();
    list.add(Prose::new("<b>init system</b>"));
    list.add(Prose::new("<b>host os</b>"));
    list.add(Prose::new("<b>evidence</b>"));
    add_node(
        &mut list,
        "<b>services</b>",
        list_from(&["name", "pid", "running status", "exit/last status"]),
    );

    section.push(list);
    section
}

fn list_from(items: &[&str]) -> UnorderedList {
    let mut list = UnorderedList::empty();
    for item in items {
        list.add(Prose::new(*item));
    }
    list
}

fn add_node(list: &mut UnorderedList, title: &str, details: UnorderedList) {
    list.add(Prose::new(title));
    list.add(details);
}
