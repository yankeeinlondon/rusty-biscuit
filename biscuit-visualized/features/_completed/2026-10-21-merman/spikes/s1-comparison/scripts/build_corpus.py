#!/usr/bin/env python3
"""Copy the hand-picked Merman fixtures, their Mermaid.js reference SVGs, and
Merman's license notices into `corpus/` (R-S1 a, R-S4, D13). Fonts are not copied.

Usage: python3 -I scripts/build_corpus.py <merman-clone> <spike-dir>
"""

import json
import shutil
import sys
from pathlib import Path

# family -> fixtures. "shared" families are those mermaid-rs-renderer 0.3.x detects
# (docs/research/mermaid/mermaid-rs-renderer.md); render results confirm or refute it.
SHARED = {
    "flowchart": [
        "upstream_cypress_flowchart_v2_spec_should_render_subgraphs_with_title_margins_and_edge_labels_063",
        "upstream_flowchart_v2_outgoing_links_2b_spec",
    ],
    "sequence": [
        "upstream_cypress_sequencediagram_spec_example_001",
        "upstream_cypress_sequencediagram_v2_spec_should_render_different_participant_types_with_notes_and_loops_015",
    ],
    "class": [
        "upstream_examples_class_basic_class_inheritance_001",
        "upstream_separators_labels_notes",
    ],
    "state": ["upstream_cypress_statediagram_v2_spec_v2_should_render_multiple_composite_states_017"],
    "er": ["upstream_examples_er_basic_er_schema_001"],
    "pie": ["upstream_docs_pie_example_003"],
    "xychart": ["upstream_cypress_xychart_spec_render_with_show_axis_label_false_015"],
    "quadrantchart": ["upstream_quadrant_docs_example_reach_engagement"],
    "sankey": ["upstream_html_demos_sankey_fy20_21_performance_001"],
    "gantt": ["zed_pr_57644_gantt", "upstream_html_demos_gantt_gantt_chart_diagram_demos_010"],
    "timeline": ["zed_pr_57644_timeline"],
    "journey": ["upstream_html_demos_journey_journey_diagram_demo_001"],
    "kanban": ["upstream_docs_kanban_full_example_012"],
    "c4": ["upstream_cypress_c4_spec_c4_1_should_render_a_simple_c4context_diagram_001"],
    "block": ["upstream_docs_block_introduction_to_block_diagrams_001"],
    "architecture": ["upstream_architecture_demo_junction_groups_arrows"],
    "requirement": ["upstream_docs_requirementdiagram_combined_example_022"],
    "mindmap": ["upstream_examples_mindmap_basic_mindmap_001"],
    "gitgraph": [
        "upstream_docs_gitgraph_merging_two_branches_009",
        "upstream_cypress_gitgraph_spec_7_should_render_a_simple_gitgraph_with_three_branches_and_tagged_007",
        "upstream_cypress_gitgraph_spec_75_should_render_a_gitgraph_with_multiple_tags_on_a_merge_commit_079",
    ],
    "zenuml": ["upstream_docs_zenuml_demo"],
    "packet": ["upstream_packet_docs_tcp_packet_example"],
    "radar": ["upstream_html_demos_radar_radar_diagram_demo_001"],
    "treemap": ["upstream_cypress_treemap_spec_2_should_render_a_hierarchical_treemap_002"],
}
MERMAN_ONLY = {
    "info": ["upstream_info_show_info_spec"],
    "venn": ["upstream_docs_venn_text_nodes_and_style_003"],
    "ishikawa": ["upstream_docs_ishikawa_basic"],
    "wardley": ["upstream_cypress_wardley_spec_3_should_render_pipelines_003"],
    "treeView": ["upstream_cypress_treeview_spec_should_render_descriptions_007"],
    "eventmodeling": ["upstream_docs_eventmodeling_minimum"],
    "cynefin": ["upstream_cypress_cynefin_spec_should_render_cynefin_without_domain_descriptions_006"],
    "railroad": ["upstream_cypress_railroad_spec_renders_sequences_and_choices_002"],
    "railroadAbnf": ["upstream_cypress_railroad_spec_renders_alternation_and_repetition_008"],
    "railroadEbnf": ["upstream_cypress_railroad_spec_renders_iso_14977_notation_007"],
    "railroadPeg": ["upstream_cypress_railroad_spec_renders_ordered_choice_and_suffixes_009"],
    "swimlane": ["upstream_docs_basic"],
    "usecase": ["shapes-and-notes"],
    "agentflow": ["container-and-shapes"],
}


def main() -> None:
    merman, spike = Path(sys.argv[1]), Path(sys.argv[2])
    fixtures = merman / "fixtures"
    out = spike / "corpus" / "merman"
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)
    manifest = []
    for group, table in (("shared", SHARED), ("merman-only", MERMAN_ONLY)):
        for family, names in table.items():
            for name in names:
                src = fixtures / family / f"{name}.mmd"
                if not src.exists():
                    raise SystemExit(f"missing fixture {src}")
                dest = out / family
                dest.mkdir(exist_ok=True)
                shutil.copy2(src, dest / src.name)
                ref = fixtures / "upstream-svgs" / family / f"{name}.svg"
                has_ref = ref.exists()
                if has_ref:
                    shutil.copy2(ref, dest / f"{name}.mermaid-js.svg")
                manifest.append({
                    "name": name,
                    "family": family,
                    "group": group,
                    "source": f"merman/fixtures/{family}/{name}.mmd",
                    "reference": f"merman/fixtures/upstream-svgs/{family}/{name}.svg" if has_ref else None,
                })
                if "theme" in src.read_text():
                    print(f"note: {family}/{name} mentions a theme in its source")
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    licenses = spike / "corpus" / "LICENSES"
    licenses.mkdir(parents=True, exist_ok=True)
    shutil.copy2(merman / "LICENSE-MIT", licenses / "merman-LICENSE-MIT")
    shutil.copy2(merman / "LICENSE-APACHE", licenses / "merman-LICENSE-APACHE")
    shutil.copytree(merman / "THIRD_PARTY_LICENSES" / "mermaid", licenses / "mermaid", dirs_exist_ok=True)
    print(f"copied {len(manifest)} fixtures")


if __name__ == "__main__":
    main()
