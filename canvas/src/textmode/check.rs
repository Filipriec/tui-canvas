// src/textmode/check.rs

#[cfg(all(feature = "textmode-vim", feature = "textmode-normal"))]
compile_error!("Enable exactly one of: textmode-vim or textmode-normal.");

#[cfg(not(any(feature = "textmode-vim", feature = "textmode-normal")))]
compile_error!("No textmode selected. Enable one of: textmode-vim or textmode-normal.");
