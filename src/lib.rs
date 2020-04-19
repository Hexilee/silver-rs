#![cfg_attr(feature = "docs", feature(doc_cfg, external_doc))]
#![cfg_attr(feature = "docs", doc(include = "../README.md"))]
#![cfg_attr(feature = "docs", warn(missing_docs))]
#![cfg_attr(not(test), deny(unsafe_code))]

pub mod net;
pub mod task;
