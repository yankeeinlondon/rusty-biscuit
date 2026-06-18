#![allow(dead_code)]

use assert_cmd::cargo::cargo_bin_cmd;
use std::io::Write;
use std::net::TcpListener;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::thread;

pub fn md_cmd() -> assert_cmd::Command {
    cargo_bin_cmd!("md")
}

pub fn md_file(content: &str) -> tempfile::NamedTempFile {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "{}", content).unwrap();
    tmp
}

pub struct MockHttpResponse {
    pub status: u16,
    pub body: &'static str,
    pub cache_control: Option<&'static str>,
}

pub struct MockHttpServer {
    base_url: String,
    requests: Arc<AtomicUsize>,
}

impl MockHttpServer {
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub fn request_count(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }
}

pub fn mock_http_server(responses: Vec<MockHttpResponse>) -> MockHttpServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let request_count = Arc::clone(&requests);

    thread::spawn(move || {
        for response in responses {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            request_count.fetch_add(1, Ordering::SeqCst);

            let mut buf = [0_u8; 4096];
            let _ = std::io::Read::read(&mut stream, &mut buf);

            let status_text = match response.status {
                200 => "OK",
                304 => "Not Modified",
                500 => "Internal Server Error",
                _ => "OK",
            };
            let mut headers = format!(
                "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n",
                response.status,
                status_text,
                response.body.len()
            );
            if let Some(cache_control) = response.cache_control {
                headers.push_str(&format!("Cache-Control: {cache_control}\r\n"));
            }
            headers.push_str("\r\n");
            let _ = stream.write_all(headers.as_bytes());
            let _ = stream.write_all(response.body.as_bytes());
        }
    });

    MockHttpServer {
        base_url: format!("http://{addr}"),
        requests,
    }
}

pub mod layout {
    use biscuit_terminal::terminal::Terminal;
    use clap::Parser;
    use darkmatter::layout::{DarkmatterPage, PageComponent};
    use darkmatter_cli::Cli;
    use darkmatter_cli::render::apply_cli_layout_flags;
    use renderable::layout::{Alignment, Edges, Length, TargetValue, Width};

    pub fn parse_cli(args: &[&str]) -> Cli {
        let mut full = vec!["md"];
        full.extend_from_slice(args);
        Cli::try_parse_from(full).expect("CLI args must parse")
    }

    pub fn resolved_page(args: &[&str]) -> DarkmatterPage {
        let cli = parse_cli(args);
        let term = Terminal::new_optimistic(120);
        apply_cli_layout_flags(DarkmatterPage::new(&term), &cli)
    }

    pub fn tv_cells(tv: &TargetValue<Length>) -> u16 {
        match tv {
            TargetValue::Universal(Length::Ch(n)) => u16::try_from(*n).unwrap_or(u16::MAX),
            _ => 0,
        }
    }

    pub fn alignment_for(page: &DarkmatterPage, component: PageComponent) -> Alignment {
        page.component_policy(component)
            .map(|p| p.layout.alignment)
            .unwrap_or_default()
    }

    #[derive(Debug, PartialEq)]
    pub enum TestFill {
        Full,
        Pad(Length),
        Indent(Length),
        Max(Length),
        Explicit(Length),
    }

    pub fn fill_for(page: &DarkmatterPage, component: PageComponent) -> TestFill {
        match page.component_policy(component) {
            None => TestFill::Full,
            Some(p) => {
                let l = &p.layout;
                if l.width == Width::Auto && l.max_width.is_none() && l.padding == Edges::default() {
                    TestFill::Full
                } else if l.width == Width::Auto
                    && l.max_width.is_none()
                    && l.padding != Edges::default()
                {
                    if l.padding.top == TargetValue::universal(Length::Zero)
                        && l.padding.bottom == TargetValue::universal(Length::Zero)
                        && l.padding.left == l.padding.right
                    {
                        TestFill::Pad(tv_length(&l.padding.left))
                    } else {
                        TestFill::Indent(tv_length(&l.padding.left))
                    }
                } else if let Some(max_width) = &l.max_width && l.width == Width::Auto {
                    TestFill::Max(tv_length(max_width))
                } else if matches!(l.width, Width::Fixed(_)) {
                    TestFill::Explicit(width_length(&l.width))
                } else {
                    TestFill::Full
                }
            }
        }
    }

    pub fn tv_length(tv: &TargetValue<Length>) -> Length {
        match tv {
            TargetValue::Universal(l) => l.clone(),
            _ => Length::Zero,
        }
    }

    pub fn width_length(w: &Width) -> Length {
        match w {
            Width::Fixed(tv) => tv_length(tv),
            _ => Length::Zero,
        }
    }

    pub fn style_prop_fixture() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("example-docs")
            .join("rendering")
            .join("style-prop.md")
    }
}

pub mod level2;
