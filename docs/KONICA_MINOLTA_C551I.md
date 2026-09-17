# Konica Minolta bizhub C551i

Use profile **konica-minolta-bizhub-c551i** from
profiles/machines/konica-minolta-bizhub-c551i.ron.

The profile uses the Konica bizhub enterprise counter layout. It has been checked
against published implementations, but has **not yet been verified on a C551i
device or its firmware**. Compare the readings with the printer's counter screen
before using them for billing.

## Setup

1. Build/run the updated application (cargo run -p printcountpay-app) or build
   the Windows installer. The installer already includes all machine profiles.
   Both the new application and profile are needed for summed click totals.
2. On the printer, open **Utility > Administrator > Network > SNMP Setting**
   (or **Network > SNMP Setting** in Web Connection).
   Enable SNMP v1/v2c(IP) and use the configured read community and UDP port
   (normally 161). PrintCountPay currently uses SNMP v2c.
3. Discover the printer. A model or system description containing C551i
   selects this profile automatically, including when the printer's display name
   has been customised. You can also select the profile explicitly in the app.
4. Poll the printer. Copy/print readings appear under Counters; scan, duplex,
   model and serial readings are labelled in Polling.

The SNMP settings above are described in the
[Konica Minolta C551i manual](https://manuals.konicaminolta.eu/bizhub-C651i-C551i-C451i/EN/contents/WC_12_03_05.html).

## Counter mapping

| Reading | OID |
| --- | --- |
| Copies B/W | 1.3.6.1.4.1.18334.1.1.1.5.7.2.2.1.5.1.1 |
| Copies colour | 1.3.6.1.4.1.18334.1.1.1.5.7.2.2.1.5.2.1 |
| Prints B/W | 1.3.6.1.4.1.18334.1.1.1.5.7.2.2.1.5.1.2 |
| Prints colour | 1.3.6.1.4.1.18334.1.1.1.5.7.2.2.1.5.2.2 |
| Device total | 1.3.6.1.4.1.18334.1.1.1.5.7.2.1.1.0 |
| Standard total fallback | 1.3.6.1.2.1.43.10.2.1.4.1.1 |
| Scan counter | 1.3.6.1.4.1.18334.1.1.1.5.7.2.3.1.5.1 |
| Duplex counter | 1.3.6.1.4.1.18334.1.1.1.5.7.2.1.3.0 |
| Model | 1.3.6.1.4.1.18334.1.1.1.1.6.2.1.0 |
| Serial number | 1.3.6.1.2.1.43.5.1.1.17.1 |

The Konica mappings come from the
[GLPI Agent Konica implementation](https://github.com/glpi-project/glpi-agent/blob/develop/lib/GLPI/Agent/SNMP/MibSupport/Konica.pm).
The four print/copy mappings also appear in
[printer-scanner](https://github.com/vvalchev/printer-scanner/blob/master/snmp_func.go).
Standard counters and supplies are defined by
[Printer-MIB / RFC 3805](https://www.rfc-editor.org/rfc/rfc3805.html).

The profile's sum_bw_color flag adds copies and prints separately for each colour
category. Every distinct component must return a numeric value; an absent
component produces an unavailable split, never an incomplete sum. Zero is a valid
reading. The total list remains an ordered fallback list and is never summed.
Existing profiles omit this flag and keep their first-available-OID behaviour.

The device total is independent and may include other output such as fax or
reports. Scan and duplex counts are diagnostic readings; they are not added to
print/copy click totals or charged by recording sessions. The scan OID is a
generic Konica scan reading, not a verified split by colour or scan destination.
Confirm full-colour, single-colour/two-colour and large-paper counter behaviour
on the installed firmware if those modes are used.

## Verify on a device

Compare the four function counts and total against the panel, then compare
before/after deltas for one B/W copy, colour copy, B/W print and colour print.
Check a scan separately. Missing private OIDs should display unavailable values,
with the standard total as fallback if it is supported.

With Net-SNMP installed, substitute the actual address and read community:

~~~text
snmpwalk -v2c -c READ_COMMUNITY -On PRINTER_IP 1.3.6.1.4.1.18334.1.1.1.5.7.2
snmpwalk -v2c -c READ_COMMUNITY -On PRINTER_IP 1.3.6.1.2.1.43
~~~

The app's **Crawl from printer** also includes the Konica counter subtree.
Recognised Konica results use the known counter mapping rather than arbitrary
numeric values such as supply levels. For a complete raw export, use the
commands above.

## Toner

Toner assignments are intentionally unset until the installed device's supply
indexes are known. The standard supply table identifies consumables by
description (1.3.6.1.2.1.43.11.1.1.6), colorant index
(1.3.6.1.2.1.43.11.1.1.3), units (1.3.6.1.2.1.43.11.1.1.7),
maximum capacity (1.3.6.1.2.1.43.11.1.1.8) and remaining level
(1.3.6.1.2.1.43.11.1.1.9). Colour names are in
1.3.6.1.2.1.43.12.1.1.4.

Use those descriptions/colorants to assign the black, cyan, magenta and yellow
level OIDs in the profile's toner section. Do not assume Ricoh's colour order.
The current UI shows the raw level; only interpret it as a percentage when the
reported unit/capacity supports that interpretation. Negative sentinel values
are not percentages.
