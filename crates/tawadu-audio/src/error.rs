//! Import failures, mapped one-to-one onto Quill's validation strings
//! (`docs/audio-ui-strings.md` §4.6 / §6). Every message is *what happened*
//! plus *what to do*; no red urgency, no invented detail.
//!
//! Bidi rule (§2): an Arabic catalogue value never contains a Latin letter.
//! The offending path travels as its own run — [`LocalizedMessage`] carries
//! the Arabic body pre-split around the placeholder so the UI can render
//! three separate runs without ever concatenating scripts.

/// Why a pack was rejected (or could not be imported).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    /// No `manifest.json` in the picked folder.
    ManifestMissing,
    /// Manifest unreadable, wrong `format`, or structurally invalid.
    ManifestInvalid { file: String },
    /// A file (or the manifest itself) has no licence entry. Hard reject.
    LicenseMissing { file: String },
    /// A file's bytes do not match the manifest's recorded sha256.
    ChecksumMismatch { file: String },
    /// The decoder cannot play this file's format.
    UnsupportedAudio { file: String },
    /// Anything else.
    Generic,
}

/// A rendered import message: English runs plus the Arabic runs pre-split
/// around the Latin placeholder (`arg`). When `arg` is empty the Arabic body
/// is `body_ar_before` alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizedMessage {
    pub title_en: String,
    pub body_en: String,
    pub title_ar: String,
    pub body_ar_before: String,
    pub arg: String,
    pub body_ar_after: String,
}

impl ImportError {
    /// The offending file, when the error names one.
    pub fn file(&self) -> Option<&str> {
        match self {
            ImportError::ManifestInvalid { file }
            | ImportError::LicenseMissing { file }
            | ImportError::ChecksumMismatch { file }
            | ImportError::UnsupportedAudio { file } => Some(file),
            _ => None,
        }
    }

    /// Quill catalogue key for the title (§4.6).
    pub fn title_key(&self) -> &'static str {
        match self {
            ImportError::ManifestMissing => "audio-error-manifest-missing-title",
            ImportError::ManifestInvalid { .. } => "audio-error-manifest-invalid-title",
            ImportError::LicenseMissing { .. } => "audio-error-license-missing-title",
            ImportError::ChecksumMismatch { .. } => "audio-error-checksum-title",
            ImportError::UnsupportedAudio { .. } => "audio-error-unsupported-audio-title",
            ImportError::Generic => "audio-error-generic-title",
        }
    }

    /// Quill catalogue key for the body (§4.6).
    pub fn body_key(&self) -> &'static str {
        match self {
            ImportError::ManifestMissing => "audio-error-manifest-missing-body",
            ImportError::ManifestInvalid { .. } => "audio-error-manifest-invalid-body",
            ImportError::LicenseMissing { .. } => "audio-error-license-missing-body",
            ImportError::ChecksumMismatch { .. } => "audio-error-checksum-body",
            ImportError::UnsupportedAudio { .. } => "audio-error-unsupported-audio-body",
            ImportError::Generic => "audio-error-generic-body",
        }
    }

    /// The exact Quill strings with the placeholder substituted. English is
    /// one Latin run; Arabic is split so the file name renders in its own run.
    pub fn message(&self) -> LocalizedMessage {
        let arg = self.file().unwrap_or_default().to_string();
        let (title_en, body_en, title_ar, before, after) = match self {
            ImportError::ManifestMissing => (
                "Manifest not found",
                format!(
                    "{arg} was not found in the pack. Choose the whole pack folder and try again."
                ),
                "ملف التعريف غير موجود",
                "لم يُعثر على ",
                " داخل الحزمة. اختر مجلد الحزمة كاملاً ثم أعد المحاولة.",
            ),
            ImportError::ManifestInvalid { .. } => (
                "Manifest is not valid",
                "The manifest could not be read. The file may be damaged, or the pack format is \
                 unsupported. Check the file and try again."
                    .to_string(),
                "ملف التعريف غير صالح",
                "تعذّرت قراءة ",
                ". تأكّد من سلامة الملف ومن أن صيغة الحزمة مدعومة، ثم أعد المحاولة.",
            ),
            ImportError::LicenseMissing { .. } => (
                "Licence entry missing",
                format!(
                    "The pack was rejected: {arg} has no licence entry. Add the licence to the \
                     pack and import again."
                ),
                "الترخيص مفقود",
                "رُفضت الحزمة: لا يوجد ترخيص للملف ",
                ". أضف الترخيص إلى الحزمة ثم أعد الاستيراد.",
            ),
            ImportError::ChecksumMismatch { .. } => (
                "Checksum mismatch",
                format!(
                    "{arg} does not match its recorded checksum; it may be corrupt or changed. \
                     Re-download the pack or replace the file and import again."
                ),
                "عدم تطابق بصمة التحقق",
                "لا يطابق الملف ",
                " بصمة التحقق المسجّلة؛ قد يكون تالفاً أو معدّلاً. أعد تنزيل الحزمة أو استبدل \
                 الملف ثم أعد الاستيراد.",
            ),
            ImportError::UnsupportedAudio { .. } => (
                "Unsupported audio format",
                format!(
                    "{arg} is in a format the app cannot play. Convert it to a supported format \
                     and import again."
                ),
                "صيغة الصوت غير مدعومة",
                "الملف ",
                " بصيغة لا يستطيع التطبيق تشغيلها. حوّله إلى صيغة مدعومة ثم أعد الاستيراد.",
            ),
            ImportError::Generic => (
                "Could not import the pack",
                "Something went wrong. Try again; if it keeps happening, check the pack files."
                    .to_string(),
                "تعذّر استيراد الحزمة",
                "حدث خطأ غير متوقع. أعد المحاولة، وإن تكرر الخطأ فتحقّق من ملفات الحزمة.",
                "",
            ),
        };
        LocalizedMessage {
            title_en: title_en.to_string(),
            body_en,
            title_ar: title_ar.to_string(),
            body_ar_before: before.to_string(),
            arg,
            body_ar_after: after.to_string(),
        }
    }
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message().body_en)
    }
}

impl std::error::Error for ImportError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn license_missing_uses_quills_exact_strings() {
        let msg = ImportError::LicenseMissing {
            file: "athan/fajr.mp3".into(),
        }
        .message();
        assert_eq!(msg.title_en, "Licence entry missing");
        assert_eq!(
            msg.body_en,
            "The pack was rejected: athan/fajr.mp3 has no licence entry. Add the licence to the \
             pack and import again."
        );
        assert_eq!(msg.title_ar, "الترخيص مفقود");
        assert_eq!(msg.body_ar_before, "رُفضت الحزمة: لا يوجد ترخيص للملف ");
        assert_eq!(msg.arg, "athan/fajr.mp3");
        assert_eq!(
            msg.body_ar_after,
            ". أضف الترخيص إلى الحزمة ثم أعد الاستيراد."
        );
        // The Arabic runs must contain no Latin letter — the file name is
        // its own run (Quill §2 rule 2).
        for run in [&msg.title_ar, &msg.body_ar_before, &msg.body_ar_after] {
            assert!(!run.chars().any(|c| c.is_ascii_alphabetic()));
        }
    }

    #[test]
    fn checksum_mismatch_names_the_file_in_both_languages() {
        let err = ImportError::ChecksumMismatch {
            file: "athan/dhuhr.mp3".into(),
        };
        assert_eq!(err.title_key(), "audio-error-checksum-title");
        assert_eq!(err.body_key(), "audio-error-checksum-body");
        let msg = err.message();
        assert!(msg.body_en.starts_with("athan/dhuhr.mp3 does not match"));
        assert_eq!(msg.arg, "athan/dhuhr.mp3");
    }

    #[test]
    fn every_variant_maps_to_a_quill_key_pair() {
        let variants = [
            ImportError::ManifestMissing,
            ImportError::ManifestInvalid {
                file: "manifest.json".into(),
            },
            ImportError::LicenseMissing {
                file: "manifest.json".into(),
            },
            ImportError::ChecksumMismatch {
                file: "a.ogg".into(),
            },
            ImportError::UnsupportedAudio {
                file: "a.xyz".into(),
            },
            ImportError::Generic,
        ];
        for err in variants {
            assert!(err.title_key().starts_with("audio-error-"));
            assert!(err.title_key().ends_with("-title"));
            assert!(err.body_key().ends_with("-body"));
            let msg = err.message();
            assert!(!msg.title_en.is_empty());
            assert!(!msg.body_en.is_empty());
            assert!(!msg.title_ar.is_empty());
            assert!(!msg.body_ar_before.is_empty());
        }
    }

    #[test]
    fn generic_error_has_no_placeholder() {
        let msg = ImportError::Generic.message();
        assert!(msg.arg.is_empty());
        assert!(msg.body_ar_after.is_empty());
    }
}
