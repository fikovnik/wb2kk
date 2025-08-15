use anyhow::{Context, Result, bail};
use chrono::DateTime;

use std::{
    collections::HashSet,
    fs::File,
    io::{self, BufWriter, Read, Write, stdout},
    path::PathBuf,
};

use clap::Parser;
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};
use serde_json::Value;
use serde_json::ser::PrettyFormatter;

const LINK_CONTENT_TYPE: &str = "link";

#[derive(Parser, Debug)]
#[command(name = "wb2kk")]
#[command(about = "Convert Wallabag export JSON to Karakeep format")]
#[command(version)]
struct Args {
    /// Input wallabag export JSON file (use '-' for stdin)
    input: String,

    /// Output file path (defaults to stdout if not specified)
    output: Option<PathBuf>,

    /// Additional tags to add to every bookmark (can be used multiple times)
    #[arg(short = 't', long = "tag")]
    tags: Vec<String>,
}

#[derive(Serialize, Debug)]
struct Content {
    #[serde(rename = "type")]
    typ: String,
    url: String,
}

#[derive(Serialize, Debug)]
struct Bookmark {
    #[serde(rename = "createdAt")]
    created_at: i64,
    title: String,
    tags: Vec<String>,
    content: Content,
    archived: bool,
    note: Option<String>,
}

struct StreamingBookmarks<'a> {
    bookmarks: &'a [Value],
    extra_tags: HashSet<String>,
}

impl<'a> StreamingBookmarks<'a> {
    fn convert_item(&self, v: &Value) -> Result<Bookmark> {
        let created_at: i64 = get_field(v, "created_at")
            .and_then(String::convert)
            .and_then(to_epoch)?;
        let title: String = get_field(v, "title").and_then(String::convert)?;
        let url: String = get_field(v, "url").and_then(String::convert)?;
        let archived: bool = get_field(v, "is_archived").and_then(i64::convert)? != 0;

        let tags: Vec<String> = {
            let xs: HashSet<String> = get_field(v, "tags")
                .and_then(Vec::convert)?
                .into_iter()
                .collect::<HashSet<String>>();
            xs.union(&self.extra_tags).cloned().collect::<Vec<String>>()
        };

        let item = Bookmark {
            created_at,
            title,
            tags,
            content: Content {
                typ: LINK_CONTENT_TYPE.to_owned(),
                url,
            },
            archived,
            note: None,
        };

        Ok(item)
    }
}

impl<'a> Serialize for StreamingBookmarks<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;

        for (i, bookmark) in self.bookmarks.iter().enumerate() {
            match self.convert_item(bookmark) {
                Ok(converted) => seq.serialize_element(&converted)?,
                Err(msg) => eprintln!("Failed to convert {i}: {msg}"),
            }
        }

        seq.end()
    }
}

fn to_epoch(s: impl AsRef<str>) -> Result<i64> {
    Ok(DateTime::parse_from_rfc3339(s.as_ref())?.timestamp())
}

trait JsonConverter<'a>: Sized {
    fn convert(v: &'a Value) -> Result<Self>;
}

impl<'a> JsonConverter<'a> for String {
    fn convert(v: &'a Value) -> Result<Self> {
        v.as_str()
            .map(<str>::to_owned)
            .with_context(|| "is not a string")
    }
}

impl<'a, T: JsonConverter<'a>> JsonConverter<'a> for Vec<T> {
    fn convert(v: &'a Value) -> Result<Self> {
        v.as_array()
            .with_context(|| "is not an array")?
            .iter()
            .map(|x| T::convert(x))
            .collect()
    }
}

impl<'a> JsonConverter<'a> for i64 {
    fn convert(v: &'a Value) -> Result<Self> {
        v.as_i64().with_context(|| "is not an int")
    }
}

fn get_field<'a>(v: &'a Value, key: &str) -> Result<&'a Value> {
    v.get(key).with_context(|| format!("{key} does not exist"))
}

fn convert(input: &str, output: &mut impl Write, tags: Vec<String>) -> Result<()> {
    let input_json: Value = serde_json::from_str(input)?;
    let Some(bookmarks) = input_json.as_array() else {
        bail!("Expected an array");
    };

    let extra_tags: HashSet<String> = tags.into_iter().collect();

    let fmt = PrettyFormatter::with_indent(b"  ");
    let mut output_json = serde_json::Serializer::with_formatter(output, fmt);
    let mut root = output_json.serialize_map(Some(1))?;

    root.serialize_entry(
        "bookmarks",
        &StreamingBookmarks {
            bookmarks,
            extra_tags,
        },
    )?;

    Ok(SerializeMap::end(root)?)
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut input = String::new();
    if args.input == "-" {
        io::stdin().read_to_string(&mut input)?;
    } else {
        File::open(&args.input)?.read_to_string(&mut input)?;
    }

    let mut output: Box<dyn Write> = if let Some(path) = &args.output {
        let file = File::create(path)?;
        Box::new(BufWriter::new(file))
    } else {
        Box::new(stdout())
    };

    convert(&input, &mut output, args.tags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn converts_wallabag_item_to_karakeep() -> Result<()> {
        let input = r#"
        [
          {
            "is_archived": 0,
            "is_starred": 0,
            "tags": [],
            "is_public": false,
            "id": 20833359,
            "title": "Linux x86 Program Start Up",
            "url": "https://web.archive.org/web/20191210114310/http://dbp-consulting.com/tutorials/debugging/linuxProgramStartup.html",
            "given_url": "https://web.archive.org/web/20191210114310/http://dbp-consulting.com/tutorials/debugging/linuxProgramStartup.html",
            "content": "Linux x86 Program Start Up\n",
            "created_at": "2025-05-15T18:45:18+02:00",
            "updated_at": "2025-05-15T18:45:18+02:00",
            "published_by": [""],
            "annotations": [],
            "reading_time": 28,
            "domain_name": "web.archive.org",
            "preview_picture": "https://web.archive.org/web/20191210114310im_/http://dbp-consulting.com/images/logo.svg"
          }
        ]"#;

        let mut output = Vec::new();
        let tags = vec!["wallabag".to_string()];

        convert(input, &mut output, tags)?;

        let produced: Value = serde_json::from_slice(&output)?;
        let expected = json!({
          "bookmarks": [
            {
              "createdAt": 1747327518,
              "title": "Linux x86 Program Start Up",
              "tags": ["wallabag"],
              "content": {
                "type": LINK_CONTENT_TYPE,
                "url": "https://web.archive.org/web/20191210114310/http://dbp-consulting.com/tutorials/debugging/linuxProgramStartup.html"
              },
              "archived": false,
              "note": null
            }
          ]
        });

        assert_eq!(produced, expected);
        Ok(())
    }
}
