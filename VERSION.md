# Release History

## Version 0.0.25

- FIX: Whastapp line now can be turned off;
- FIX: More error logging on BuildMin.


## Version 0.0.24

- FIX: AskDialog compilation error.
- FIX: More Whatsapp line adjustments: Duplicated 'Hi!' & log enrichment.

## Version 0.0.23

- FIX: Duplicated asking on main loop removed.
- FIX: Whatsapp log phrase correction.
- FIX: Directline can now receive messages sent in not-in-conversation, projector-only fashion.

## Version 0.0.22

- NEW: Auto-dispatch to dialog based on intent name.

## Version 0.0.21

- FIX: Whatsapp directline client improved.

## Version 0.0.20

- NEW: Whatsapp directline client is now working in preview.

## Version 0.0.19

- NEW: Whatsapp directline client started.
- NEW: Console directline client.
- NEW: Now each .gbapp has it own set of syspackages loaded.
- NEW: Added support for Whatsapp external service key on bot instance model.

## Version 0.0.18
- FIX: .gbapp files now correctly loaded before other package types so custom models can be used to sync DB.
- NEW: Removed Boot Package feature. Now every .gbot found on deploy folders are deployed on startup.
