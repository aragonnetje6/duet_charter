use color_eyre::eyre::{eyre, Result};

use crate::chart::LyricEvent;
use crate::chart::TimestampedEvent;

#[derive(Debug)]
pub struct PhraseLyric {
    timestamp: u32,
    text: String,
}

#[derive(Debug)]
pub struct Phrase {
    start_timestamp: u32,
    end_timestamp: u32,
    lyrics: Vec<PhraseLyric>,
}

pub fn phraseify(lyrics: &[LyricEvent]) -> Result<Vec<Phrase>> {
    lyrics
        .iter()
        .filter(|elem| {
            !matches!(
                elem,
                LyricEvent::Section { .. } | LyricEvent::Default { .. }
            )
        })
        .collect::<Vec<_>>()
        .split_inclusive(|elem| matches!(elem, LyricEvent::PhraseStart { .. }))
        .filter(|semi_phrase| semi_phrase.len() > 1)
        .map(|mut semi_phrase| {
            if let LyricEvent::PhraseStart { .. } = semi_phrase
                .first()
                .ok_or_else(|| eyre!("Empty semi-phrase {:?}", semi_phrase))?
            {
                semi_phrase = semi_phrase
                    .split_first()
                    .ok_or_else(|| eyre!("Semi-phrase split failed {:?}", semi_phrase))?
                    .1;
            }
            let start_timestamp = semi_phrase
                .first()
                .ok_or_else(|| eyre!("Empty semi-phrase {:?}", semi_phrase))?
                .get_timestamp();
            let end_timestamp = match semi_phrase
                .last()
                .ok_or_else(|| eyre!("Empty semi-phrase {:?}", semi_phrase))?
            {
                LyricEvent::PhraseStart { timestamp } => timestamp - 1,
                LyricEvent::PhraseEnd { timestamp } => *timestamp,
                LyricEvent::Lyric { timestamp, .. } => timestamp + 1,
                _ => unreachable!(),
            };
            let lyrics: Vec<PhraseLyric> = semi_phrase
                .iter()
                .filter_map(|elem| match elem {
                    LyricEvent::Lyric { timestamp, text } => Some(PhraseLyric {
                        timestamp: *timestamp,
                        text: text.clone(),
                    }),
                    _ => None,
                })
                .collect();
            Ok(Phrase {
                start_timestamp,
                end_timestamp,
                lyrics,
            })
        })
        .collect()
}

#[cfg(test)]
mod test {
    use std::fs;
    use std::io::Read;

    use color_eyre::eyre::WrapErr;
    use indicatif::ProgressBar;

    use crate::chart::Chart;

    use super::*;

    #[test]
    fn phrase_test() -> Result<()> {
        let dir: Vec<_> = fs::read_dir("./charts/")?.collect();
        let bar = ProgressBar::new(dir.len() as u64);
        for folder in dir {
            bar.inc(1);
            let entry = folder?;
            phrase_test_helper(&entry).wrap_err(format!(
                "Error occurred for chart file {}",
                &entry.file_name().to_str().unwrap_or("filename failure")
            ))?;
        }
        bar.finish();
        Ok(())
    }

    fn phrase_test_helper(folder: &fs::DirEntry) -> Result<()> {
        let mut path = folder.path();
        path.push("notes");
        path.set_extension("chart");
        let mut file = fs::File::open(&path)?;
        let mut file_content = String::new();
        file.read_to_string(&mut file_content)?;
        let chart = Chart::from(&file_content)?;
        phraseify(&chart.get_lyrics())?;
        Ok(())
    }
}
