pub(crate) const SYS_DESCR_OID: [u32; 9] = [1, 3, 6, 1, 2, 1, 1, 1, 0];
pub(crate) const SYS_OBJECT_ID_OID: [u32; 9] = [1, 3, 6, 1, 2, 1, 1, 2, 0];
pub(crate) const SYS_NAME_OID: [u32; 9] = [1, 3, 6, 1, 2, 1, 1, 5, 0];
pub(crate) const SYS_UPTIME_OID: [u32; 9] = [1, 3, 6, 1, 2, 1, 1, 3, 0];
pub(crate) const PRT_GENERAL_PRINTER_NAME_OID: [u32; 12] = [
    1, 3, 6, 1, 2, 1, 43, 5, 1, 1, 16, 1,
];
pub(crate) const PRT_MARKER_LIFECOUNT_1: [u32; 13] = [
    1, 3, 6, 1, 2, 1, 43, 10, 2, 1, 4, 1, 1,
];
pub(crate) const PRT_MARKER_LIFECOUNT_2: [u32; 13] = [
    1, 3, 6, 1, 2, 1, 43, 10, 2, 1, 4, 1, 2,
];
pub(crate) const PRT_MARKER_LIFECOUNT_3: [u32; 13] = [
    1, 3, 6, 1, 2, 1, 43, 10, 2, 1, 4, 1, 3,
];
pub(crate) const RICOH_COUNTER_ROOT: [u32; 12] = [1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 19];
pub(crate) const RICOH_TONER_ROOT: [u32; 12] = [1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 24];
pub(crate) const RICOH_COLOR_COPIER_COUNT_OID: [u32; 16] = [
    1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 19, 5, 1, 9, 17,
];
pub(crate) const RICOH_COLOR_PRINTER_COUNT_OID: [u32; 16] = [
    1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 19, 5, 1, 9, 60,
];
pub(crate) const RICOH_BW_COPIER_COUNT_OID: [u32; 16] = [
    1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 19, 5, 1, 9, 18,
];
pub(crate) const RICOH_BW_PRINTER_COUNT_OID: [u32; 16] = [
    1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 19, 5, 1, 9, 61,
];
pub(crate) const RICOH_TONER_BLACK_OID: [u32; 16] = [
    1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 24, 1, 1, 5, 1,
];
pub(crate) const RICOH_TONER_CYAN_OID: [u32; 16] = [
    1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 24, 1, 1, 5, 2,
];
pub(crate) const RICOH_TONER_MAGENTA_OID: [u32; 16] = [
    1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 24, 1, 1, 5, 3,
];
pub(crate) const RICOH_TONER_YELLOW_OID: [u32; 16] = [
    1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 24, 1, 1, 5, 4,
];
pub(crate) const PRINTER_MIB_ROOT: [u32; 7] = [1, 3, 6, 1, 2, 1, 43];
pub(crate) const RICOH_MIB_ROOT: [u32; 7] = [1, 3, 6, 1, 4, 1, 367];
pub(crate) const CRAWL_ROOTS: [&[u32]; 4] = [
    &PRINTER_MIB_ROOT,
    &RICOH_MIB_ROOT,
    &RICOH_COUNTER_ROOT,
    &RICOH_TONER_ROOT,
];
pub(crate) const DISCOVERY_CONCURRENCY: usize = 24;
pub(crate) const MAX_VARBINDS_SHOWN: usize = 200;
pub(crate) const FALLBACK_DISCOVERY_CIDR: &str = "192.168.129.1/24";
