use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use claudine::services::protect::catalog::RuleGroup;
use claudine::services::protect::config::{ProtectRuleToggles, RuleGroupConfig};

use super::super::app::{App, AppMode, ModalState};
use super::super::widgets::toggle::Toggle;

// ---------------------------------------------------------------------------
// Category grouping for the protect rules grid
// ---------------------------------------------------------------------------

struct ProtectCategory {
    name: &'static str,
    groups: &'static [RuleGroup],
}

fn protect_categories() -> [ProtectCategory; 4] {
    [
        ProtectCategory {
            name: "Filesystem",
            groups: &[
                RuleGroup::FilesystemDestruction,
                RuleGroup::DiskManipulation,
                RuleGroup::SensitivePaths,
            ],
        },
        ProtectCategory {
            name: "Network",
            groups: &[
                RuleGroup::RemoteExecution,
                RuleGroup::NetworkSabotage,
                RuleGroup::CredentialExfiltration,
            ],
        },
        ProtectCategory {
            name: "System",
            groups: &[
                RuleGroup::SystemSabotage,
                RuleGroup::ContainerCloud,
                RuleGroup::DatabaseNukes,
            ],
        },
        ProtectCategory {
            name: "Code Safety",
            groups: &[
                RuleGroup::GitDestructive,
                RuleGroup::ObfuscatedExecution,
                RuleGroup::PromptInjection,
            ],
        },
    ]
}

fn rule_display_name(group: RuleGroup) -> &'static str {
    match group {
        RuleGroup::FilesystemDestruction => "File Destruction",
        RuleGroup::DiskManipulation => "Disk Manipulation",
        RuleGroup::RemoteExecution => "Remote Execution",
        RuleGroup::GitDestructive => "Git Destructive",
        RuleGroup::SystemSabotage => "System Sabotage",
        RuleGroup::NetworkSabotage => "Network Sabotage",
        RuleGroup::ContainerCloud => "Container/Cloud",
        RuleGroup::DatabaseNukes => "Database Nukes",
        RuleGroup::ObfuscatedExecution => "Obfuscated Exec",
        RuleGroup::PromptInjection => "Prompt Injection",
        RuleGroup::CredentialExfiltration => "Credential Exfil",
        RuleGroup::SensitivePaths => "Sensitive Paths",
        RuleGroup::Custom => "Custom",
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_group_enabled(rules: &ProtectRuleToggles, group: RuleGroup) -> bool {
    match rules.get(group) {
        None => true,
        Some(RuleGroupConfig::Toggle(enabled)) => *enabled,
        Some(RuleGroupConfig::Detailed(d)) => d.enabled,
    }
}

fn is_default_config(rules: &ProtectRuleToggles) -> bool {
    RuleGroup::all_builtin()
        .iter()
        .all(|g| rules.get(*g).is_none())
}

fn count_enabled_rules(rules: &ProtectRuleToggles) -> (usize, usize) {
    let groups = RuleGroup::all_builtin();
    let total = groups.len();
    let enabled = groups
        .iter()
        .filter(|g| is_group_enabled(rules, **g))
        .count();
    (enabled, total)
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let is_detail = app.mode == AppMode::Detail;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Logging toggle
            Constraint::Length(1), // blank separator
            Constraint::Length(1), // Protect toggle
            Constraint::Min(0),   // Protect detail (when enabled)
        ])
        .split(area);

    let logging_toggle = Toggle::new("Logging", app.config.logging, is_detail);
    frame.render_widget(logging_toggle, chunks[0]);

    let protect_enabled = app.config.protect.enabled;
    let protect_toggle = Toggle::new("Protect", protect_enabled, is_detail);
    frame.render_widget(protect_toggle, chunks[2]);

    if protect_enabled {
        render_protect_detail(frame, chunks[3], &app.config.protect.rules);
    }

    if let Some(ModalState::ProtectRules { highlighted, staged_rules }) = &app.modal {
        render_protect_rules_modal(frame, area, staged_rules, *highlighted);
    }
}

fn render_protect_detail(frame: &mut Frame, area: Rect, rules: &ProtectRuleToggles) {
    if area.height < 3 {
        return;
    }

    let is_default = is_default_config(rules);
    let tree_indent: u16 = 4;
    let desc_indent: u16 = tree_indent + 5;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // │
            Constraint::Length(1), // ╰── heading
            Constraint::Length(1), // blank
            Constraint::Length(3), // description
            Constraint::Length(1), // blank
            Constraint::Min(0),   // category grid
        ])
        .split(area);

    // Vertical connector: │
    let pipe = Paragraph::new(Line::from(vec![
        Span::raw(" ".repeat(tree_indent as usize)),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(pipe, chunks[0]);

    // Rounded elbow: ╰── Default/Custom Configuration
    let heading_label = if is_default {
        "Default Configuration"
    } else {
        "Custom Configuration"
    };
    let heading = Paragraph::new(Line::from(vec![
        Span::raw(" ".repeat(tree_indent as usize)),
        Span::styled("╰── ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            heading_label,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    frame.render_widget(heading, chunks[1]);

    // Description (indented past the tree connector)
    let desc_area = Rect {
        x: area.x + desc_indent,
        width: area.width.saturating_sub(desc_indent),
        ..chunks[3]
    };

    if is_default {
        let desc = Paragraph::new(
            "The default configuration represents a sensible defensive posture against \
             shell commands and MCP injection attacks but tries to maintain a balance so \
             that flow is not interrupted.",
        )
        .style(
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )
        .wrap(Wrap { trim: false });
        frame.render_widget(desc, desc_area);
    } else {
        let (enabled, total) = count_enabled_rules(rules);
        let desc = Paragraph::new(Line::from(vec![
            Span::styled(
                "You are using ",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ),
            Span::styled(format!("{enabled}"), Style::default().fg(Color::Yellow)),
            Span::styled(
                format!(" of the {total} protection patterns that "),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ),
            Span::styled(
                "Claudine",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC | Modifier::BOLD),
            ),
            Span::styled(
                " provides.",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]))
        .wrap(Wrap { trim: false });
        frame.render_widget(desc, desc_area);
    }

    // Category grid — only render if enough terminal space
    let grid_area = Rect {
        x: area.x + desc_indent,
        width: area.width.saturating_sub(desc_indent),
        ..chunks[5]
    };
    if grid_area.width >= 72 && grid_area.height >= 5 {
        render_category_columns(frame, grid_area, rules);
    }
}

fn render_category_columns(frame: &mut Frame, area: Rect, rules: &ProtectRuleToggles) {
    let categories = protect_categories();

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .split(area);

    for (i, cat) in categories.iter().enumerate() {
        let item_count = cat.groups.len();
        let constraints: Vec<Constraint> = std::iter::once(Constraint::Length(1))
            .chain((0..item_count).map(|_| Constraint::Length(1)))
            .chain(std::iter::once(Constraint::Min(0)))
            .collect();

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(cols[i]);

        // Category header
        let header = Paragraph::new(Span::styled(
            cat.name,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(header, rows[0]);

        // Rule items
        for (j, &group) in cat.groups.iter().enumerate() {
            let enabled = is_group_enabled(rules, group);
            let (icon, icon_style) = if enabled {
                ("✓", Style::default().fg(Color::Green))
            } else {
                ("⤫", Style::default().fg(Color::Red))
            };
            let item = Paragraph::new(Line::from(vec![
                Span::styled(icon, icon_style),
                Span::raw(" "),
                Span::styled(rule_display_name(group), Style::default().fg(Color::Gray)),
            ]));
            frame.render_widget(item, rows[j + 1]);
        }
    }
}

// ---------------------------------------------------------------------------
// Protect rules modal (unchanged)
// ---------------------------------------------------------------------------

fn render_protect_rules_modal(
    frame: &mut Frame,
    area: Rect,
    staged_rules: &ProtectRuleToggles,
    highlighted: usize,
) {
    let rule_names = super::super::get_protect_rule_names();
    let enabled_rules = staged_rules;

    super::super::widgets::modal::render_modal(
        frame,
        area,
        "Protect Rules",
        50,
        70,
        |frame, area| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(1), // blank separator
                    Constraint::Length(1), // hotkey bar
                ])
                .split(area);

            let items: Vec<ListItem> = rule_names
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let is_enabled = super::super::is_protect_rule_enabled(enabled_rules, name);
                    let check = if is_enabled { "[x]" } else { "[ ]" };
                    let style = if i == highlighted {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    ListItem::new(Line::from(Span::styled(format!("{check} {name}"), style)))
                })
                .collect();

            let list = List::new(items)
                .highlight_symbol(">> ")
                .highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                );
            let mut state = ListState::default().with_selected(Some(highlighted));
            frame.render_stateful_widget(list, chunks[0], &mut state);

            let hotkey_line = super::super::widgets::modal::build_modal_hotkey_line(&[
                ("SPACE", "Toggle"),
                ("ENTER", "Done"),
                ("ESC", "Cancel"),
            ]);
            let hotkey_widget = Paragraph::new(hotkey_line)
                .alignment(Alignment::Center)
                .style(Style::default().bg(Color::Indexed(236)));
            frame.render_widget(hotkey_widget, chunks[2]);
        },
    );
}

// ---------------------------------------------------------------------------
// Key handling (unchanged)
// ---------------------------------------------------------------------------

pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('l') | KeyCode::Char('L') => {
            app.config.logging = !app.config.logging;
            app.dirty = true;
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            app.config.protect.enabled = !app.config.protect.enabled;
            app.dirty = true;
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.modal = Some(ModalState::ProtectRules {
                highlighted: 0,
                staged_rules: app.config.protect.rules.clone(),
            });
        }
        _ => {}
    }
}

pub fn handle_protect_rules_modal(app: &mut App, key: KeyEvent) {
    let rule_names = super::super::get_protect_rule_names();
    let count = rule_names.len();
    match key.code {
        KeyCode::Up => {
            let idx = app.modal_highlighted();
            if idx > 0 {
                app.set_modal_highlighted(idx - 1);
            }
        }
        KeyCode::Down => {
            let idx = app.modal_highlighted();
            if idx + 1 < count {
                app.set_modal_highlighted(idx + 1);
            }
        }
        KeyCode::Char(' ') => {
            let idx = app.modal_highlighted();
            if let Some(name) = rule_names.get(idx) {
                if let Some(ModalState::ProtectRules { staged_rules, .. }) = &mut app.modal {
                    super::super::toggle_protect_rule(staged_rules, name);
                }
            }
        }
        KeyCode::Enter => {
            // Commit staged rules to config
            if let Some(ModalState::ProtectRules { staged_rules, .. }) = app.modal.take() {
                app.config.protect.rules = staged_rules;
                app.dirty = true;
            }
        }
        KeyCode::Esc => {
            // Discard staged changes
            app.modal = None;
        }
        _ => {}
    }
}
