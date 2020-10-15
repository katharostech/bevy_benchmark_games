fn main() {
    cfg_aliases::cfg_aliases! {
        headless: { not(feature = "with_graphics") }
    }
}
