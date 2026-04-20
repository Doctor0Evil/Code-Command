// FILE: ./coreengine/src/tokenwalker.rs
#[derive(Copy, Clone, Debug)]
pub enum LanguageHint {
    Rust = 0,
    Js   = 1,
    Cpp  = 2,
    Aln  = 3,
    Md   = 4,
}

pub const BIT_CC_FILE: u32 = 1 << 0;
pub const BIT_CC_LANG: u32 = 1 << 1;
pub const BIT_CC_FULL: u32 = 1 << 2;
pub const BIT_CC_PATH: u32 = 1 << 3;
pub const BIT_CC_DEEP: u32 = 1 << 4;
pub const BIT_CC_SOV:  u32 = 1 << 5;
pub const BIT_CC_NAV:  u32 = 1 << 6;
pub const BIT_CC_VOL:  u32 = 1 << 7;
pub const BIT_CC_CRATE:u32 = 1 << 8;

// LanguageHint encoded in bits 16..19.
const LANG_SHIFT: u32 = 16;
const LANG_MASK:  u32 = 0xF << LANG_SHIFT;

// Blacklist flags (pattern classes) in bits 20..23.
pub const BIT_BL_SOV_CRATES: u32 = 1 << 20; // reqwest, serde_json, openai, etc.[file:2][file:4]
pub const BIT_BL_BLACKLIST:  u32 = 1 << 21; // (*/) hard blacklist names.

#[derive(Copy, Clone, Debug)]
pub struct ScanProfile {
    pub bits: u32,
}

impl ScanProfile {
    pub fn for_tags_and_lang(tags: &[String], lang: LanguageHint) -> Self {
        let mut bits = 0u32;
        for t in tags {
            match t.as_str() {
                "CC-FILE"  => bits |= BIT_CC_FILE,
                "CC-LANG"  => bits |= BIT_CC_LANG,
                "CC-FULL"  => bits |= BIT_CC_FULL,
                "CC-PATH"  => bits |= BIT_CC_PATH,
                "CC-DEEP"  => bits |= BIT_CC_DEEP,
                "CC-SOV"   => { bits |= BIT_CC_SOV | BIT_BL_SOV_CRATES; }
                "CC-NAV"   => bits |= BIT_CC_NAV,
                "CC-VOL"   => bits |= BIT_CC_VOL,
                "CC-CRATE" => bits |= BIT_CC_CRATE,
                _ => {}
            }
        }
        bits |= (lang as u32) << LANG_SHIFT;
        // If blacklist (*/ ) items are configured in policy, also set BIT_BL_BLACKLIST.[file:3][file:4]
        ScanProfile { bits }
    }

    pub fn language(&self) -> LanguageHint {
        let v = (self.bits & LANG_MASK) >> LANG_SHIFT;
        match v {
            0 => LanguageHint::Rust,
            1 => LanguageHint::Js,
            2 => LanguageHint::Cpp,
            3 => LanguageHint::Aln,
            4 => LanguageHint::Md,
            _ => LanguageHint::Rust,
        }
    }

    #[inline]
    pub fn has(&self, mask: u32) -> bool {
        (self.bits & mask) != 0
    }
}
