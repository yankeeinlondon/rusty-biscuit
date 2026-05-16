use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MicrodataKey {
    Title,
    Description,
    Author,
    Image,
    Url,
    SiteName,
    Locale,
    Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetaSource {
    Html,
    OpenGraph,
    Twitter,
    SchemaOrg,
}

pub type MetaHtml = String;
pub type MicrodataMap = HashMap<MetaSource, Vec<MetaHtml>>;

pub fn microdata(key: MicrodataKey, value: impl AsRef<str>) -> MicrodataMap {
    let value = escape_html_attr(value.as_ref());

    let mut out: MicrodataMap = HashMap::new();

    match key {
        MicrodataKey::Title => {
            push(
                &mut out,
                MetaSource::Html,
                format!(r#"<title>{}</title>"#, escape_html_text(&value)),
            );
            push(
                &mut out,
                MetaSource::OpenGraph,
                meta_property("og:title", &value),
            );
            push(
                &mut out,
                MetaSource::Twitter,
                meta_name("twitter:title", &value),
            );
            push(
                &mut out,
                MetaSource::SchemaOrg,
                meta_itemprop("name", &value),
            );
        }

        MicrodataKey::Description => {
            push(&mut out, MetaSource::Html, meta_name("description", &value));
            push(
                &mut out,
                MetaSource::OpenGraph,
                meta_property("og:description", &value),
            );
            push(
                &mut out,
                MetaSource::Twitter,
                meta_name("twitter:description", &value),
            );
            push(
                &mut out,
                MetaSource::SchemaOrg,
                meta_itemprop("description", &value),
            );
        }

        MicrodataKey::Author => {
            push(&mut out, MetaSource::Html, meta_name("author", &value));
            push(
                &mut out,
                MetaSource::SchemaOrg,
                meta_itemprop("author", &value),
            );
        }

        MicrodataKey::Image => {
            push(
                &mut out,
                MetaSource::OpenGraph,
                meta_property("og:image", &value),
            );
            push(
                &mut out,
                MetaSource::Twitter,
                meta_name("twitter:image", &value),
            );
            push(
                &mut out,
                MetaSource::SchemaOrg,
                meta_itemprop("image", &value),
            );
        }

        MicrodataKey::Url => {
            push(
                &mut out,
                MetaSource::OpenGraph,
                meta_property("og:url", &value),
            );
            push(
                &mut out,
                MetaSource::Twitter,
                meta_name("twitter:url", &value),
            );
            push(
                &mut out,
                MetaSource::SchemaOrg,
                meta_itemprop("url", &value),
            );
        }

        MicrodataKey::SiteName => {
            push(
                &mut out,
                MetaSource::OpenGraph,
                meta_property("og:site_name", &value),
            );
        }

        MicrodataKey::Locale => {
            push(
                &mut out,
                MetaSource::OpenGraph,
                meta_property("og:locale", &value),
            );
        }

        MicrodataKey::Type => {
            push(
                &mut out,
                MetaSource::OpenGraph,
                meta_property("og:type", &value),
            );
        }
    }

    out
}

fn push(map: &mut MicrodataMap, source: MetaSource, html: MetaHtml) {
    map.entry(source).or_default().push(html);
}

fn meta_name(name: &str, content: &str) -> String {
    format!(r#"<meta name="{name}" content="{content}">"#)
}

fn meta_property(property: &str, content: &str) -> String {
    format!(r#"<meta property="{property}" content="{content}">"#)
}

fn meta_itemprop(itemprop: &str, content: &str) -> String {
    format!(r#"<meta itemprop="{itemprop}" content="{content}">"#)
}

fn escape_html_attr(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_html_text(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
