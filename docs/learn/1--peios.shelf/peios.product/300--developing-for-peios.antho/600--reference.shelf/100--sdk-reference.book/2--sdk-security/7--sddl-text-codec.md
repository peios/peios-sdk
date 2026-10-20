---
title: SDDL Text Codec
description: Converting between the binary forms and human-readable SDDL text, entirely in userspace, including conditional expressions.
---

The SDDL codec converts between the binary wire forms and their
human-readable SDDL text (MS-DTYP §2.5.1). This is a **pure-userspace
facility** — the kernel speaks only binary — so it lives entirely in libpeios.
All four entries use the two-call protocol (`cap == 0` to probe) and fail with
`EINVAL` on malformed input.

```c
ssize_t peios_sddl_parse_sd(void *out, size_t cap, const char *sddl);
ssize_t peios_sddl_format_sd(char *out, size_t cap, const void *sd, size_t sd_len);
```

- `peios_sddl_parse_sd` parses SDDL text (for example
  `"O:SYG:BAD:(A;;FA;;;BA)"`)
  into self-relative SD wire bytes.
- `peios_sddl_format_sd` renders SD wire bytes back to a NUL-terminated SDDL
  string (length excludes the `NUL`, so allocate `len + 1`).

These are the friendliest way to construct a descriptor when you have one
written down — parse the string rather than assembling ACEs by hand — and the
friendliest way to log or display one.

## Conditional expressions

Callback ACEs carry a *conditional expression* as compiled "artx" bytecode.
The codec converts between that bytecode and its SDDL expression text:

```c
ssize_t peios_sddl_parse_condition(void *out, size_t cap, const char *expr);
ssize_t peios_sddl_format_condition(char *out, size_t cap, const void *artx, size_t len);
```

- `peios_sddl_parse_condition` compiles an expression such as `@User.Title ==
  "PM"` into the bytecode you place in a callback ACE's `app_data`.
- `peios_sddl_format_condition` renders bytecode back to text (with no outer
  parentheses), length excluding the `NUL`.

So the round trip for a conditional ACE is: write the condition as text →
`peios_sddl_parse_condition` → put the bytecode in `peios_ace_spec.app_data`
with a callback ACE `type` → add it to an ACL builder.

## Input limits and malformed data

Conditional expressions have a maximum tree depth of 128. Each comparison,
existence check, membership test, logical operator, or composite operand adds
one level; scalar operands add none. For example, a comparison has depth one,
and adding 127 negations reaches the limit. Long `&&` and `||` chains also
count because each operator adds a level to the resulting tree.

Text parsing also limits nested parentheses, negations, and composite braces
to 128 levels along any one parsing path. Delimiters inside string literals do
not count. These limits apply when parsing conditional ACEs inside a
descriptor as well as when converting a standalone condition. The bytecode
decoder applies the same tree limit, including nested composite operands.

Inputs beyond either limit fail with `EINVAL`; simplify the expression before
retrying. The C functions return `-1` and leave the output buffer untouched.

GUID fields require ASCII hexadecimal digits in the standard hyphenated form.
Malformed Unicode GUID text fails with `EINVAL`. When reading resource claims,
the codec checks that the complete value-offset table fits inside the supplied
bytes before allocating storage for its entries. An impossible or truncated
table also fails with `EINVAL`.

## Descriptor text and omitted components

Sections use `O:<sid>G:<sid>D:<acl>S:<acl>`. Each is optional but can appear
only once. Formatting orders them as owner, group, DACL, and SACL.

`D:` is an empty, present DACL. `D:NO_ACCESS_CONTROL` and an omitted DACL
encode the absent-DACL state. The marker can accompany ACL flags but not
ACEs. The same representation rule applies to SACL absence. The formatter
can omit an absent ACL unless flags need preserving, and uses
`D:NO_ACCESS_CONTROL` for an otherwise empty descriptor.

These rules describe a complete supplied descriptor. In a tool that queried
only selected components, an omitted section can instead mean "not queried."
Formatting cannot recover missing components. Do not use matching partial
SDDL strings as proof of complete descriptor equality or as a full backup.

An ACE has six semicolon-separated fields, or seven for a payload:

```text
(type;flags;rights;object_guid;inherit_object_guid;account_sid;payload)
```

Object GUID fields apply to object ACEs; supplying them for other types is
an error. Callback payloads carry conditions, and resource-attribute payloads
carry claims. For example, `O:SYG:SYD:(A;OICI;GA;;;SY)` names SYSTEM as owner
and group and grants SYSTEM generic-all with object/container inheritance.
This explains the string; it does not select a suitable policy for every
object.

## ACL control flags

| Flag | Meaning |
| --- | --- |
| `P` | Protected. The ACL does not inherit from its parent. |
| `AI` | Auto-inherited. The ACL was produced by inheritance propagation. |
| `AR` | Auto-inherit required. Requests that propagation be applied. |

## ACE type codes

| Code | ACE type |
| --- | --- |
| `A` | Access allowed |
| `D` | Access denied |
| `AU` | System audit |
| `OA` | Access allowed, object |
| `OD` | Access denied, object |
| `OU` | System audit, object |
| `XA` | Access allowed, callback — carries a condition |
| `XD` | Access denied, callback — carries a condition |
| `XU` | System audit, callback — carries a condition |
| `ZA` | Access allowed, callback object |
| `ZD` | Access denied, callback object |
| `ZU` | System audit, callback object |
| `AL` | System alarm — continuous auditing |
| `OL` | System alarm, object |
| `XL` | System alarm, callback — carries a condition |
| `ZL` | System alarm, callback object |
| `ML` | Mandatory label — the integrity level |
| `TL` | Process trust label |
| `RA` | Resource attribute — carries a claim |
| `SP` | Scoped policy identifier |
| `FL` | Access filter — carries a condition |

## ACE flag codes

| Code | Meaning |
| --- | --- |
| `CI` | Container inherit — child containers receive it. |
| `OI` | Object inherit — child objects receive it. |
| `NP` | No propagate — children receive it, grandchildren do not. |
| `IO` | Inherit only — it does not apply to the object carrying it. |
| `ID` | Inherited — this ACE arrived by propagation. |
| `SA` | Audit successful access. |
| `FA` | Audit failed access. |

## Rights codes

| Code | Right |
| --- | --- |
| `GA` | Generic all |
| `GR` | Generic read |
| `GW` | Generic write |
| `GX` | Generic execute |

| Code | Right |
| --- | --- |
| `SD` | Delete |
| `RC` | Read control — read the descriptor |
| `WD` | Write DACL |
| `WO` | Write owner |

| Code | Mask | Right |
| --- | --- | --- |
| `FA` | `0x001F01FF` | All file access |
| `FR` | `0x00120089` | File read |
| `FW` | `0x00120116` | File write |
| `FX` | `0x001200A0` | File execute |

| Code | Mask | Right |
| --- | --- | --- |
| `KA` | `0x000F003F` | All key access |
| `KR` | `0x00020019` | Key read |
| `KW` | `0x00020006` | Key write |
| `KX` | `0x00020019` | Key execute |

| Code | Right |
| --- | --- |
| `CC` | Create child |
| `DC` | Delete child |
| `LC` | List children |
| `SW` | Self write |
| `RP` | Read property |
| `WP` | Write property |
| `DT` | Delete tree |
| `LO` | List object |
| `CR` | Control access |

| Code | Policy |
| --- | --- |
| `NW` | No write up |
| `NR` | No read up |
| `NX` | No execute up |

## Absolute SID aliases

| Code | SID | Principal |
| --- | --- | --- |
| `WD` | `S-1-1-0` | Everyone |
| `AN` | `S-1-5-7` | Anonymous |
| `IU` | `S-1-5-4` | Interactive |
| `NU` | `S-1-5-2` | Network logon |
| `SU` | `S-1-5-6` | Service |
| `RC` | `S-1-5-12` | Restricted code |
| `WR` | `S-1-5-33` | Write-restricted code |
| `PS` | `S-1-5-10` | Principal self |
| `ED` | `S-1-5-9` | Enterprise domain controllers |
| `AU` | `S-1-5-11` | Authenticated Users |
| `SY` | `S-1-5-18` | Local System |
| `LS` | `S-1-5-19` | Local Service |
| `NS` | `S-1-5-20` | Network Service |
| `AC` | `S-1-15-2-1` | All application packages |

| Code | SID | Principal |
| --- | --- | --- |
| `CO` | `S-1-3-0` | Creator Owner |
| `CG` | `S-1-3-1` | Creator Group |
| `OW` | `S-1-3-4` | Owner rights |

| Code | SID | Principal |
| --- | --- | --- |
| `LW` | `S-1-16-4096` | Low integrity |
| `ME` | `S-1-16-8192` | Medium integrity |
| `MP` | `S-1-16-8448` | Medium-plus integrity |
| `HI` | `S-1-16-12288` | High integrity |
| `SI` | `S-1-16-16384` | System integrity |

| Code | SID | Principal |
| --- | --- | --- |
| `BA` | `S-1-5-32-544` | Administrators |
| `BU` | `S-1-5-32-545` | Users |
| `BG` | `S-1-5-32-546` | Guests |
| `PU` | `S-1-5-32-547` | Power users |
| `AO` | `S-1-5-32-548` | Account operators |
| `SO` | `S-1-5-32-549` | Server operators |
| `PO` | `S-1-5-32-550` | Printer operators |
| `BO` | `S-1-5-32-551` | Backup operators |
| `RE` | `S-1-5-32-552` | Replicator |
| `RU` | `S-1-5-32-554` | Pre-Windows 2000 compatible access |
| `RD` | `S-1-5-32-555` | Remote desktop users |
| `NO` | `S-1-5-32-556` | Network configuration operators |
| `MU` | `S-1-5-32-558` | Performance monitor users |
| `LU` | `S-1-5-32-559` | Performance log users |
| `IS` | `S-1-5-32-568` | IIS users |
| `CY` | `S-1-5-32-569` | Cryptographic operators |
| `ER` | `S-1-5-32-573` | Event log readers |
| `CD` | `S-1-5-32-574` | Certificate service DCOM access |
| `RA` | `S-1-5-32-575` | RDS remote access servers |
| `ES` | `S-1-5-32-576` | RDS endpoint servers |
| `MS` | `S-1-5-32-577` | RDS management servers |
| `HA` | `S-1-5-32-578` | Hyper-V administrators |
| `AA` | `S-1-5-32-579` | Access control assistance operators |
| `RM` | `S-1-5-32-580` | Remote management users |

## Context-sensitive codes

| Code | As a right | As an account | Elsewhere |
| --- | --- | --- | --- |
| `WD` | Write DACL | Everyone | |
| `RC` | Read control | Restricted code | |
| `AU` | | Authenticated Users | ACE type: system audit |
| `RA` | | RDS remote access servers | ACE type: resource attribute |
| `FA` | All file access | | ACE flag: audit failed access |
| `SD` | Delete | | |

## Numeric masks and canonical formatting

An entirely ASCII-digit rights field is an unsigned decimal mask. Leading
zeros remain decimal; signs, non-ASCII digits, overflow beyond `4294967295`,
and mixing decimal text with mnemonic codes are rejected. Hexadecimal uses
`0x` and consumes the rest of the field, so it can follow mnemonic rights
only as the final portion. `FRFX0x6` combines file read/execute with the two
additional bits.

The formatter prefers composite masks and does not preserve every accepted
spelling. `KX` has the same mask as `KR`; one spelling is emitted. Directory
object codes share bits with other vocabularies, so outside mandatory-label
ACEs formatting can use hexadecimal rather than guess an object class.
Mandatory-label masks use their label vocabulary. Unknown ACE types fail
formatting instead of silently disappearing.

`XL` and `ZL` are Peios callback-alarm extensions and are not portable Windows
ACE codes. Domain-relative aliases such as `DA` and machine-relative `LA`
cannot resolve without a base SID; the codec rejects them with an explanation.
Use an explicit full SID. BUILTIN aliases are absolute despite their names.
The null SID and protected-process integrity SID have no short alias.

A displayed `f` in the `sd` command's rights summary is that command's
shorthand, not the SDDL grammar or a hexadecimal nibble. Consult the tool's
manual before feeding rendered output back into a parser.
