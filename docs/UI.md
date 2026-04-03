# UI Design

## Overall Layout

```
┌─────────────────────────────────────────────────────────┐
│  [C2PA Tool]          [Sign] [Verify] [Settings]    [?] │
├─────────────────────────────────────────────────────────┤
│                                                         │
│                    < page content >                     │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

Top nav with three sections: **Sign**, **Verify**, **Settings**.

---

## Sign Page

```
┌─────────────────────────────────────────────────────────┐
│  Sign Asset                                             │
├───────────────────────┬─────────────────────────────────┤
│                       │  Manifest Definition            │
│   ┌───────────────┐   │  ┌───────────────────────────┐  │
│   │               │   │  │ Title: __________________ │  │
│   │  Drop file    │   │  │ Format: _________________ │  │
│   │  here or      │   │  │                           │  │
│   │  [Browse]     │   │  │ Assertions:               │  │
│   │               │   │  │  + Add assertion          │  │
│   └───────────────┘   │  └───────────────────────────┘  │
│   video.mp4  ✓        │                                 │
│                       │  Ingredients                    │
│   Options             │  ┌───────────────────────────┐  │
│   ○ Embed manifest    │  │  (empty)     [+ Add]      │  │
│   ○ External manifest │  └───────────────────────────┘  │
│   ○ Fragmented        │                                 │
│                       │  Signer                         │
│                       │  ┌───────────────────────────┐  │
│                       │  │ Certificate: [Browse]     │  │
│                       │  │ Key:         [Browse]     │  │
│                       │  └───────────────────────────┘  │
│                       │                                 │
│                       │          [Sign Asset]           │
└───────────────────────┴─────────────────────────────────┘
```

---

## Verify Page

```
┌─────────────────────────────────────────────────────────┐
│  Verify Asset                                           │
├───────────────────────┬─────────────────────────────────┤
│                       │  Validation Result              │
│   ┌───────────────┐   │                                 │
│   │               │   │  ┌───────────────────────────┐  │
│   │  Drop file    │   │  │  ● VERIFIED               │  │
│   │  here or      │   │  │                           │  │
│   │  [Browse]     │   │  │  Signed by:               │  │
│   │               │   │  │    Adobe Inc.             │  │
│   └───────────────┘   │  │                           │  │
│   image.jpg  ✓        │  │  Timestamp:               │  │
│                       │  │    2026-03-15 10:42 UTC   │  │
│   Also check for:     │  │                           │  │
│   ☑ External manifest │  │  Trust: ✓ Trusted CA      │  │
│   ☑ Remote manifest   │  └───────────────────────────┘  │
│                       │                                 │
│                       │  Manifest                       │
│                       │  ┌───────────────────────────┐  │
│                       │  │ ▶ active_manifest          │  │
│                       │  │   ▶ assertions (3)         │  │
│                       │  │   ▶ ingredients (1)        │  │
│                       │  │     └─ original.jpg ✓      │  │
│                       │  └───────────────────────────┘  │
│                       │                                 │
│                       │  [Export Report]                │
└───────────────────────┴─────────────────────────────────┘
```

Validation state visually distinct:

```
● VERIFIED      — green
● TAMPERED      — red
● UNVERIFIABLE  — amber
● UNSIGNED      — gray
```

---

## Settings Page

```
┌─────────────────────────────────────────────────────────┐
│  Settings                                               │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Trust Lists                                            │
│  ┌─────────────────────────────────────────────────┐   │
│  │  c2pa-trust-list.pem          [Remove]          │   │
│  │  custom-ca.pem                [Remove]          │   │
│  │                               [+ Add]           │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  Configuration                                          │
│  ┌─────────────────────────────────────────────────┐   │
│  │  ○ Load from file   [config.toml]  [Browse]     │   │
│  │  ○ Load from JSON   [____________________]      │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  HTTP Resolution                                        │
│  ┌─────────────────────────────────────────────────┐   │
│  │  ☑ Fetch remote manifests automatically         │   │
│  │  Timeout: [30] seconds                          │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│                              [Save]  [Reset to Default] │
└─────────────────────────────────────────────────────────┘
```

---

## Key Design Decisions

- **Two-panel layout** on Sign/Verify: file input on the left, configuration/results on the right — keeps the workflow linear
- **Validation state badge** is the hero element on the Verify page — immediately visible
- **Tree view for manifests** — reflects the hierarchical ingredient chain from the C2PA spec
- **Settings are global** — trust lists and config apply to both signing and verification
- **Fragmented content** handled as an option on the Sign page rather than a separate page — keeps the surface area small
