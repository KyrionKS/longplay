//! Tag reading on top of lofty.

use std::path::Path;

use lofty::file::TaggedFileExt;
use lofty::picture::PictureType;
use lofty::prelude::{Accessor, AudioFile};
use lofty::tag::ItemKey;

#[derive(Debug, Clone, Default)]
pub struct FileTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track: Option<u32>,
    pub disc: Option<u32>,
    pub year: Option<u16>,
    pub genre: Option<String>,
    pub label: Option<String>,
    pub duration: f64,
    pub picture: Option<Vec<u8>>,
}

#[derive(Debug, thiserror::Error)]
pub enum MetadataError {
    #[error("cannot read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: lofty::error::FileParseError,
    },
}

pub fn read(path: &Path) -> Result<FileTags, MetadataError> {
    let tagged = lofty::read_from_path(path).map_err(|source| MetadataError::Read {
        path: path.display().to_string(),
        source,
    })?;

    let duration = tagged.properties().duration().as_secs_f64();
    let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) else {
        return Ok(FileTags {
            duration,
            ..FileTags::default()
        });
    };

    let text = |key: ItemKey| tag.get_string(key).map(ToOwned::to_owned);

    let picture = tag
        .pictures()
        .iter()
        .find(|p| p.pic_type() == PictureType::CoverFront)
        .or_else(|| tag.pictures().first())
        .map(|p| p.data().to_vec());

    Ok(FileTags {
        title: tag.title().map(|v| v.into_owned()),
        artist: tag.artist().map(|v| v.into_owned()),
        album: tag.album().map(|v| v.into_owned()),
        album_artist: text(ItemKey::AlbumArtist),
        track: tag.track(),
        disc: tag.disk(),
        year: tag.date().map(|d| d.year),
        genre: tag.genre().map(|v| v.into_owned()),
        label: text(ItemKey::Label),
        duration,
        picture,
    })
}

pub fn duration(path: &Path) -> Result<f64, MetadataError> {
    let tagged = lofty::read_from_path(path).map_err(|source| MetadataError::Read {
        path: path.display().to_string(),
        source,
    })?;
    Ok(tagged.properties().duration().as_secs_f64())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_an_error() {
        let result = read(Path::new("/definitely/not/here.flac"));
        assert!(result.is_err());
    }
}
