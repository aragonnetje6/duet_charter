use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use clap::Parser;
use eyre::Result;

use lyric_charter_lib::chart::Chart;
use lyric_charter_lib::phrases::LyricPhrases;

/// Commandline lyric charting tool for Clone Hero .chart files!
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Source .chart file to make into duet
    #[clap(value_parser)]
    source: String,

    /// Destination to save result to
    #[clap(value_parser)]
    dest: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let source_str = String::from(&args.source);
    let dest_str = String::from(&args.dest.unwrap_or_else(|| "duet.chart".to_owned()));

    let source = Path::new(&source_str);
    let dest = Path::new(&dest_str);

    let mut file = fs::File::open(source)?;
    let mut file_str = String::new();
    file.read_to_string(&mut file_str)?;
    let chart = Chart::from(&file_str)?;
    let phrases = LyricPhrases::new(chart.get_lyrics());
    println!("main: {:?}", phrases.get_main_phrases());
    println!("duet: {:?}", phrases.get_duet_phrases());
    let mut out_file = fs::File::create(dest)?;
    let byte_count_main = out_file.write(
        phrases
            .get_main_phrases()
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<String>>()
            .join("\r\n")
            .as_bytes(),
    )?;
    let byte_count_duet = out_file.write(
        phrases
            .get_duet_phrases()
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<String>>()
            .join("\r\n")
            .as_bytes(),
    )?;
    println!("{} bytes written", byte_count_main + byte_count_duet);

    Ok(())
}
