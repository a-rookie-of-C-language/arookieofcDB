pub struct PageLayout;

impl PageLayout {
    pub const HEADER_OFFSET: usize = 0;
    pub const HEADER_SIZE: usize = 11;

    // For Internal Page
    pub const INTERNAL_PAGE_IDS_OFFSET: usize = Self::HEADER_SIZE;
    // For Leaf Page
    pub const LEAF_NEXT_PAGE_ID_OFFSET: usize = 7; // In Header
}
