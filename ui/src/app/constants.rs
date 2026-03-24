pub(crate) const SYS_DESCR_OID: [u32; 9] = [1, 3, 6, 1, 2, 1, 1, 1, 0];
pub(crate) const SYS_OBJECT_ID_OID: [u32; 9] = [1, 3, 6, 1, 2, 1, 1, 2, 0];
pub(crate) const SYS_NAME_OID: [u32; 9] = [1, 3, 6, 1, 2, 1, 1, 5, 0];
pub(crate) const SYS_UPTIME_OID: [u32; 9] = [1, 3, 6, 1, 2, 1, 1, 3, 0];
pub(crate) const PRT_GENERAL_PRINTER_NAME_OID: [u32; 12] = [1, 3, 6, 1, 2, 1, 43, 5, 1, 1, 16, 1];
pub(crate) const PRT_MARKER_LIFECOUNT_1: [u32; 13] = [1, 3, 6, 1, 2, 1, 43, 10, 2, 1, 4, 1, 1];
pub(crate) const PRT_MARKER_LIFECOUNT_2: [u32; 13] = [1, 3, 6, 1, 2, 1, 43, 10, 2, 1, 4, 1, 2];
pub(crate) const PRT_MARKER_LIFECOUNT_3: [u32; 13] = [1, 3, 6, 1, 2, 1, 43, 10, 2, 1, 4, 1, 3];
pub(crate) const RICOH_COUNTER_ROOT: [u32; 12] = [1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 19];
pub(crate) const RICOH_COUNTER_VALUE_ROOT: [u32; 15] =
    [1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 19, 5, 1, 9];
pub(crate) const RICOH_TONER_ROOT: [u32; 12] = [1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 24];
pub(crate) const RICOH_COLOR_COPIER_COUNT_OID: [u32; 16] =
    [1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 19, 5, 1, 9, 203];
pub(crate) const RICOH_COLOR_PRINTER_COUNT_OID: [u32; 16] =
    [1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 19, 5, 1, 9, 403];
pub(crate) const RICOH_BW_COPIER_COUNT_OID: [u32; 16] =
    [1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 19, 5, 1, 9, 201];
pub(crate) const RICOH_BW_PRINTER_COUNT_OID: [u32; 16] =
    [1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 19, 5, 1, 9, 401];
pub(crate) const RICOH_TONER_BLACK_OID: [u32; 16] =
    [1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 24, 1, 1, 5, 1];
pub(crate) const RICOH_TONER_CYAN_OID: [u32; 16] =
    [1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 24, 1, 1, 5, 2];
pub(crate) const RICOH_TONER_MAGENTA_OID: [u32; 16] =
    [1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 24, 1, 1, 5, 3];
pub(crate) const RICOH_TONER_YELLOW_OID: [u32; 16] =
    [1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 24, 1, 1, 5, 4];
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

#[derive(Debug, Clone, Copy)]
pub(crate) struct RicohCounterEntry {
    pub(crate) type_id: u32,
    pub(crate) unit: &'static str,
    pub(crate) label: &'static str,
}

pub(crate) const RICOH_COUNTER_TABLE: [RicohCounterEntry; 81] = [
    RicohCounterEntry {
        type_id: 10,
        unit: "sheets",
        label: "Counter: Machine Total",
    },
    RicohCounterEntry {
        type_id: 200,
        unit: "sheets",
        label: "Counter: Copy: Total",
    },
    RicohCounterEntry {
        type_id: 201,
        unit: "sheets",
        label: "Counter: Copy: Black & White",
    },
    RicohCounterEntry {
        type_id: 202,
        unit: "sheets",
        label: "Counter: Copy: Single/Two-color",
    },
    RicohCounterEntry {
        type_id: 203,
        unit: "sheets",
        label: "Counter: Copy: Full Color",
    },
    RicohCounterEntry {
        type_id: 300,
        unit: "sheets",
        label: "Counter: FAX: Total",
    },
    RicohCounterEntry {
        type_id: 301,
        unit: "sheets",
        label: "Counter: FAX: Black & White",
    },
    RicohCounterEntry {
        type_id: 400,
        unit: "sheets",
        label: "Counter: Print: Total",
    },
    RicohCounterEntry {
        type_id: 401,
        unit: "sheets",
        label: "Counter: Print: Black & White",
    },
    RicohCounterEntry {
        type_id: 402,
        unit: "sheets",
        label: "Counter: Print: Single/Two-col.",
    },
    RicohCounterEntry {
        type_id: 403,
        unit: "sheets",
        label: "Counter: Print: Full Color",
    },
    RicohCounterEntry {
        type_id: 10,
        unit: "sheets",
        label: "Counter: Machine Total",
    },
    RicohCounterEntry {
        type_id: 600,
        unit: "sheets",
        label: "Total Prints: Full Color",
    },
    RicohCounterEntry {
        type_id: 601,
        unit: "sheets",
        label: "Total Prints: Monocolor",
    },
    RicohCounterEntry {
        type_id: 602,
        unit: "sheets",
        label: "Development: Color",
    },
    RicohCounterEntry {
        type_id: 603,
        unit: "sheets",
        label: "Development: Black & White",
    },
    RicohCounterEntry {
        type_id: 604,
        unit: "sheets",
        label: "Copier: Color",
    },
    RicohCounterEntry {
        type_id: 605,
        unit: "sheets",
        label: "Copier: Black & White",
    },
    RicohCounterEntry {
        type_id: 606,
        unit: "sheets",
        label: "Printer: Color",
    },
    RicohCounterEntry {
        type_id: 607,
        unit: "sheets",
        label: "Printer: Black & White",
    },
    RicohCounterEntry {
        type_id: 608,
        unit: "sheets",
        label: "Total Prints: Color",
    },
    RicohCounterEntry {
        type_id: 609,
        unit: "sheets",
        label: "Total Prints: Black & White",
    },
    RicohCounterEntry {
        type_id: 610,
        unit: "sheets",
        label: "Total Prints: Full Color A3",
    },
    RicohCounterEntry {
        type_id: 611,
        unit: "sheets",
        label: "Total Prints: Full Color <= B4",
    },
    RicohCounterEntry {
        type_id: 612,
        unit: "sheets",
        label: "Printer: Full Color",
    },
    RicohCounterEntry {
        type_id: 613,
        unit: "sheets",
        label: "Printer: Monocolor",
    },
    RicohCounterEntry {
        type_id: 614,
        unit: "sheets",
        label: "Total Prints: Full Color (GPC)",
    },
    RicohCounterEntry {
        type_id: 617,
        unit: "sheets",
        label: "Total Prints: Specific Two-color",
    },
    RicohCounterEntry {
        type_id: 660,
        unit: "sheets",
        label: "Total Prints: Full Color wo-sp2C",
    },
    RicohCounterEntry {
        type_id: 661,
        unit: "sheets",
        label: "Total Prints: Monocolor wo-sp2C",
    },
    RicohCounterEntry {
        type_id: 662,
        unit: "sheets",
        label: "Printer: Full Color wo-sp2C",
    },
    RicohCounterEntry {
        type_id: 620,
        unit: "sheets",
        label: "Copier: Black & White",
    },
    RicohCounterEntry {
        type_id: 621,
        unit: "sheets",
        label: "Copier: Single Color",
    },
    RicohCounterEntry {
        type_id: 622,
        unit: "sheets",
        label: "Copier: Two-color",
    },
    RicohCounterEntry {
        type_id: 623,
        unit: "sheets",
        label: "Copier: Full Color",
    },
    RicohCounterEntry {
        type_id: 630,
        unit: "sheets",
        label: "Fax: Black & White",
    },
    RicohCounterEntry {
        type_id: 631,
        unit: "sheets",
        label: "Fax: Single Color",
    },
    RicohCounterEntry {
        type_id: 640,
        unit: "sheets",
        label: "Printer: Black & White",
    },
    RicohCounterEntry {
        type_id: 641,
        unit: "sheets",
        label: "Printer: 1 or 2 Clr. Toner(s)",
    },
    RicohCounterEntry {
        type_id: 642,
        unit: "sheets",
        label: "Printer: Full Color",
    },
    RicohCounterEntry {
        type_id: 644,
        unit: "sheets",
        label: "Printer: Single Color",
    },
    RicohCounterEntry {
        type_id: 643,
        unit: "sheets",
        label: "Printer: Two-color",
    },
    RicohCounterEntry {
        type_id: 650,
        unit: "sheets",
        label: "From Storage: Black & White",
    },
    RicohCounterEntry {
        type_id: 651,
        unit: "sheets",
        label: "From Storage: Single Color",
    },
    RicohCounterEntry {
        type_id: 652,
        unit: "sheets",
        label: "From Storage: Two-color",
    },
    RicohCounterEntry {
        type_id: 653,
        unit: "sheets",
        label: "From Storage: Full Color",
    },
    RicohCounterEntry {
        type_id: 700,
        unit: "sheets",
        label: "Large Paper Prints: >= A3, DLT",
    },
    RicohCounterEntry {
        type_id: 701,
        unit: "times",
        label: "No. of Printed Sides in Duplex",
    },
    RicohCounterEntry {
        type_id: 900,
        unit: "sheets",
        label: "Total Jobs: All Applications",
    },
    RicohCounterEntry {
        type_id: 901,
        unit: "sheets",
        label: "Total Jobs: Copier Application",
    },
    RicohCounterEntry {
        type_id: 902,
        unit: "sheets",
        label: "Total Jobs: Fax Application",
    },
    RicohCounterEntry {
        type_id: 903,
        unit: "sheets",
        label: "Total Jobs: Printer Application",
    },
    RicohCounterEntry {
        type_id: 904,
        unit: "sheets",
        label: "Total Jobs: Scanner Application",
    },
    RicohCounterEntry {
        type_id: 905,
        unit: "sheets",
        label: "Total Jobs: Storage Application",
    },
    RicohCounterEntry {
        type_id: 906,
        unit: "sheets",
        label: "Total Jobs: Other Application",
    },
    RicohCounterEntry {
        type_id: 800,
        unit: "sheets",
        label: "Counter: Machine Total",
    },
    RicohCounterEntry {
        type_id: 810,
        unit: "sheets",
        label: "Copier: Full Color",
    },
    RicohCounterEntry {
        type_id: 811,
        unit: "sheets",
        label: "Copier: Black & White",
    },
    RicohCounterEntry {
        type_id: 812,
        unit: "sheets",
        label: "Copier: Single Color",
    },
    RicohCounterEntry {
        type_id: 813,
        unit: "sheets",
        label: "Copier: Two-color",
    },
    RicohCounterEntry {
        type_id: 820,
        unit: "sheets",
        label: "Printer: Full Color",
    },
    RicohCounterEntry {
        type_id: 821,
        unit: "sheets",
        label: "Printer: Black & White",
    },
    RicohCounterEntry {
        type_id: 822,
        unit: "sheets",
        label: "Printer: Single Color",
    },
    RicohCounterEntry {
        type_id: 823,
        unit: "sheets",
        label: "Printer: Two-color",
    },
    RicohCounterEntry {
        type_id: 830,
        unit: "sheets",
        label: "Fax: Black & White",
    },
    RicohCounterEntry {
        type_id: 831,
        unit: "sheets",
        label: "Fax: Single Color",
    },
    RicohCounterEntry {
        type_id: 840,
        unit: "sheets",
        label: "Large Paper Prints: >= A3, DLT",
    },
    RicohCounterEntry {
        type_id: 841,
        unit: "times",
        label: "No. of Printed Sides in Duplex",
    },
    RicohCounterEntry {
        type_id: 850,
        unit: "percent",
        label: "Coverage: Color",
    },
    RicohCounterEntry {
        type_id: 851,
        unit: "percent",
        label: "Coverage: Black & White",
    },
    RicohCounterEntry {
        type_id: 852,
        unit: "sheets",
        label: "Coverage: Color Print Page",
    },
    RicohCounterEntry {
        type_id: 853,
        unit: "sheets",
        label: "Coverage: B/W Print Page",
    },
    RicohCounterEntry {
        type_id: 862,
        unit: "sheets",
        label: "Total Prints: Full Color (GPC)",
    },
    RicohCounterEntry {
        type_id: 870,
        unit: "sheets",
        label: "Counter: Transmission: Total",
    },
    RicohCounterEntry {
        type_id: 871,
        unit: "sheets",
        label: "Counter: Transmission: B/W",
    },
    RicohCounterEntry {
        type_id: 872,
        unit: "sheets",
        label: "Counter: Transmission: FAX",
    },
    RicohCounterEntry {
        type_id: 873,
        unit: "sheets",
        label: "Counter: Transmission: Color Scan",
    },
    RicohCounterEntry {
        type_id: 874,
        unit: "sheets",
        label: "Counter: Transmission: B/W Scan",
    },
    RicohCounterEntry {
        type_id: 854,
        unit: "sheets",
        label: "Color Coverage & Distribution 1",
    },
    RicohCounterEntry {
        type_id: 855,
        unit: "sheets",
        label: "Color Coverage & Distribution 2",
    },
    RicohCounterEntry {
        type_id: 856,
        unit: "sheets",
        label: "Color Coverage & Distribution 3",
    },
];
