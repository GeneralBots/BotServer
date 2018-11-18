## [1.0.7](https://github.com/pragmatismo-io/BotServer/compare/1.0.6...1.0.7) (2018-11-18)


### Bug Fixes

* **config:** CHANGELOG generator fixing. ([ac18782](https://github.com/pragmatismo-io/BotServer/commit/ac18782))

## Version 0.1.9 (Before CI with Semantic Release)

* Republishing.

## Version 0.1.8

* Republishing.

## Version 0.1.7

* 100% automated development environement setup.
* Azure Deployer based on ARM done - setup is easy as F5 in Visual Studio.
* Auto-ngrok - No more reverse proxy manual configuration.
* Strategy to replicate itself in several subscriptions done.

## Version 0.1.6

* Updated packages references.

## Version 0.1.5

* Updated packages references.

## Version 0.1.4

* Error handling improved and logging enriched as well.
* Setting DATABASE_ is now STORAGE_.


## Version 0.1.3

* FIX: Admin now is internationalized.
* FIX: Webchat now receives a private token.
* FIX: OAuth2 now has got revised and included state to avoid CSRF attacks.
* FIX: Now server will only start with a secure administration password.

## Version 0.1.2

* NEW: kb.gbapp now has a complete browser of excel articles.
* FIX: Some security improved.
* NEW: Protocol changes for exchanging questions between UI and Bot Server.

## Version 0.1.0

- NEW: Migration to Bot Framework v4.

## Version 0.0.31

- FIX: Updated dependencies versions.

## Version 0.0.30

- FIX: Packages updated.
- NEW: DATABASE_SYNC_ALTER environment parameter.
- NEW: DATABASE_SYNC_FORCE environment parameter.
- NEW: Define constraint names in MSSQL.

## Version 0.0.29

- NEW: Added STT and TTS capabilities to default.gbui.

## Version 0.0.28

- FIX: gbui packages updated.

## Version 0.0.27

- FIX: Packages updated.

## Version 0.0.26

- FIX: Packages updated.
- NEW: If a bot package's name begins with '.', then it is ignored.
- NEW: Created DATABASE_LOGGING environment parameter.

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
