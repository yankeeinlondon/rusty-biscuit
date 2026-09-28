use mermaid_rs_renderer::{parse_mermaid, compute_layout, render_svg, LayoutConfig, Theme};
use mermaid_rs_renderer::layout::DiagramData;
use std::{fs, path::Path};
fn main() {
    let out = std::env::args().nth(1).expect("output directory");
    let out = Path::new(&out); fs::create_dir_all(out).unwrap();
    let adjacent = "gitGraph\ncommit id: \"A\" tag: \"feature/very-long-exact-branch-reference-alpha\"\ncommit id: \"B\" tag: \"origin/very-long-exact-branch-reference-beta\"\n";
    let merge = "gitGraph\ncommit id: \"FORK\"\nbranch feature\ncheckout feature\ncommit id: \"TIP\" tag: \"feature/merged-branch\"\ncheckout main\ncommit id: \"PARENT\"\nmerge feature id: \"MERGE\" tag: \"origin/main\" tag: \"main-at-merge\"\ncommit id: \"AFTER\"\n";
    let old = "gitGraph\ncommit id: \"FORK\"\nbranch feature\ncheckout feature\ncommit id: \"+999\" type: HIGHLIGHT\ncommit id: \"TIP\" tag: \"feature/old-merged-branch\"\ncheckout main\ncommit id: \"+4999\" type: HIGHLIGHT\ncommit id: \"PARENT\"\nmerge feature id: \"MERGE\" tag: \"origin/main\" tag: \"main-at-merge\"\ncommit id: \"+2999\" type: HIGHLIGHT\ncommit id: \"NOW\" tag: \"main\"\n";
    let mut report = String::new();
    for (name, source, repair, spacing) in [("adjacent-default",adjacent,false,false),("adjacent-spaced",adjacent,false,true),("merge-default",merge,false,false),("merge-repaired",merge,true,true),("old-compressed-repaired",old,true,true)] {
        fs::write(out.join(format!("{name}.mmd")),source).unwrap();
        let mut graph = parse_mermaid(source).unwrap().graph;
        if repair {
            let c=graph.gitgraph.commits.iter_mut().find(|c| c.id=="MERGE").unwrap();
            assert_eq!(c.parents,vec!["PARENT"]);
            c.parents.push("TIP".into());
        }
        let theme=Theme::mermaid_default(); let mut config=LayoutConfig::default();
        let initial=compute_layout(&graph,&theme,&config);
        if spacing {
            let DiagramData::GitGraph(g)=&initial.diagram else {panic!()};
            let widest=g.commits.iter().flat_map(|c| &c.tags).map(|t| {let min=t.points.iter().map(|p|p.0).fold(f32::INFINITY,f32::min);let max=t.points.iter().map(|p|p.0).fold(f32::NEG_INFINITY,f32::max);max-min}).fold(0f32,f32::max);
            config.gitgraph.commit_step=config.gitgraph.commit_step.max(widest+16.0);
        }
        let layout=compute_layout(&graph,&theme,&config);
        let DiagramData::GitGraph(g)=&layout.diagram else {panic!()};
        let mut boxes=Vec::new();
        for c in &g.commits {for t in &c.tags {
            assert!(t.transform.is_none());
            let x1=t.points.iter().map(|p|p.0).fold(f32::INFINITY,f32::min);let x2=t.points.iter().map(|p|p.0).fold(f32::NEG_INFINITY,f32::max);
            let y1=t.points.iter().map(|p|p.1).fold(f32::INFINITY,f32::min);let y2=t.points.iter().map(|p|p.1).fold(f32::NEG_INFINITY,f32::max);
            assert!(graph.gitgraph.commits.iter().find(|original| original.id == c.id).unwrap().tags.contains(&t.text));
            boxes.push((&c.id,&t.text,x1,y1,x2,y2));
        }}
        let mut collisions=0;
        for (i,a) in boxes.iter().enumerate() {for b in boxes.iter().skip(i+1) {if a.2<b.4 && b.2<a.4 && a.3<b.5 && b.3<a.5 {collisions+=1;}}}
        if spacing {assert_eq!(collisions,0);}
        let svg=render_svg(&layout,&theme,&config); fs::write(out.join(format!("{name}.svg")),&svg).unwrap();
        mermaid_rs_renderer::render::write_output_png(&svg,&out.join(format!("{name}.png")),&Default::default(),&theme).unwrap();
        report.push_str(&format!("{name}: commits={}, step={}, dimensions={}x{}, tag_collisions={}\n",g.commits.len(),config.gitgraph.commit_step,g.width,g.height,collisions));
        for c in &graph.gitgraph.commits {report.push_str(&format!("  {} parents={:?} tags={:?}\n",c.id,c.parents,c.tags));}
        for b in boxes {report.push_str(&format!("  tag_box={b:?}\n"));}
        if repair {let c=graph.gitgraph.commits.iter().find(|c|c.id=="MERGE").unwrap();assert_eq!(c.parents,vec!["PARENT","TIP"]);assert_eq!(c.tags,vec!["origin/main","main-at-merge"]);}
        if name=="old-compressed-repaired" {assert_eq!(g.commits.len(),8);assert!(g.commits.iter().any(|c|c.id=="FORK"));assert!(g.commits.iter().any(|c|c.id=="MERGE"));}
    }
    fs::write(out.join("results.txt"),&report).unwrap(); print!("{report}");
}
