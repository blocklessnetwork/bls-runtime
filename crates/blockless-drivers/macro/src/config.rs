use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, Result, Token};
use wiggle_generate::config::{Paths, WitxConf};

pub struct BlocklessConfig {
    pub c: WitxConf,
    pub target: syn::Path,
}

enum BlocklessConfigField {
    WitxField(Paths),
    TargetField(syn::Path),
}

mod kw {
    syn::custom_keyword!(witx);
    syn::custom_keyword!(target);
}

/// The Blockless Configure for the Witx File, use Witx genrate the code of linker abi.
impl BlocklessConfig {
    fn build(fields: impl Iterator<Item = BlocklessConfigField>) -> Result<Self> {
        let mut witx_confg = None;
        let mut target = None;
        for f in fields {
            match f {
                BlocklessConfigField::TargetField(t) => target = Some(t),
                BlocklessConfigField::WitxField(paths) => {
                    witx_confg = Some(WitxConf::Paths(paths));
                }
            }
        }
        let bc = BlocklessConfig {
            c: witx_confg.take().expect("witx is not set."),
            target: target.take().expect("target is not set."),
        };
        Ok(bc)
    }

    pub fn load_document(&self) -> witx::Document {
        self.c.load_document()
    }
}

impl Parse for BlocklessConfig {
    fn parse(input: ParseStream) -> Result<Self> {
        let contents;
        let _ = braced!(contents in input);
        let fields: Punctuated<BlocklessConfigField, Token![,]> =
            contents.parse_terminated(BlocklessConfigField::parse)?;
        Self::build(fields.into_iter())
    }
}

impl Parse for BlocklessConfigField {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::witx) {
            input.parse::<kw::witx>()?;
            input.parse::<Token![:]>()?;
            Ok(BlocklessConfigField::WitxField(input.parse()?))
        } else if lookahead.peek(kw::target) {
            input.parse::<kw::target>()?;
            input.parse::<Token![:]>()?;
            Ok(BlocklessConfigField::TargetField(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}
