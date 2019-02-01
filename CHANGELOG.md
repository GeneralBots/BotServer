## [1.2.2](https://github.com/pragmatismo-io/BotServer/compare/1.2.1...1.2.2) (2019-02-01)


### Bug Fixes

* **auth:** setupSecurity now is a complete setup process for tokens. ([4718fe4](https://github.com/pragmatismo-io/BotServer/commit/4718fe4))
* **deployer:** Installs and compiles additional .gbapps on server startup. ([cfe5cd2](https://github.com/pragmatismo-io/BotServer/commit/cfe5cd2))
* **kb.gbapp:** Menu and Ask dialog flows fixing. ([d884bc3](https://github.com/pragmatismo-io/BotServer/commit/d884bc3))
* **VBA:** Removal of invalid error messages. ([dd92032](https://github.com/pragmatismo-io/BotServer/commit/dd92032))

## [1.2.1](https://github.com/pragmatismo-io/BotServer/compare/1.2.0...1.2.1) (2018-12-18)


### Bug Fixes

* **kb:** Fix in Faq and Menu dialogs. ([6ba8c09](https://github.com/pragmatismo-io/BotServer/commit/6ba8c09))
* **startup:** Startup improved and more checks added. ([5d6c60e](https://github.com/pragmatismo-io/BotServer/commit/5d6c60e))
* **webchat:** Sync versions and MSFT strategy. ([238c0bf](https://github.com/pragmatismo-io/BotServer/commit/238c0bf))

# [1.2.0](https://github.com/pragmatismo-io/BotServer/compare/1.1.1...1.2.0) (2018-12-13)


### Features

* **webchat:** Update of webchat to the newer version 4. ([0270a8e](https://github.com/pragmatismo-io/BotServer/commit/0270a8e))

## [1.1.1](https://github.com/pragmatismo-io/BotServer/compare/1.1.0...1.1.1) (2018-12-08)


### Bug Fixes

* **package:** update csv-parse to version 4.1.0 ([a606ef1](https://github.com/pragmatismo-io/BotServer/commit/a606ef1))

# [1.1.0](https://github.com/pragmatismo-io/BotServer/compare/1.0.8...1.1.0) (2018-12-06)


### Bug Fixes

* **CI:** default.gbui compilation issues. ([7a11919](https://github.com/pragmatismo-io/BotServer/commit/7a11919))
* **CI:** Migrating CI logic to package.json. ([8ee048f](https://github.com/pragmatismo-io/BotServer/commit/8ee048f))
* **core:** Bot boot logic being fixed. ([1761e06](https://github.com/pragmatismo-io/BotServer/commit/1761e06))
* **core:** Bot Server is runnable again after refactory. ([9379dec](https://github.com/pragmatismo-io/BotServer/commit/9379dec))
* **core:** Loaded dynamically a .js file containing converted VBA dialogs. ([3f32e48](https://github.com/pragmatismo-io/BotServer/commit/3f32e48))
* **core:** Moved logic from app to core. ([c1db8be](https://github.com/pragmatismo-io/BotServer/commit/c1db8be))
* **default.gbui:** Removing warnings. ([02ed085](https://github.com/pragmatismo-io/BotServer/commit/02ed085))
* **gbdialog:** Renamed alpha command to alpha-VBA added documentation files. ([9cd66b8](https://github.com/pragmatismo-io/BotServer/commit/9cd66b8))
* **gbdialog:** Support for multiples hear blocks. ([3bb9d65](https://github.com/pragmatismo-io/BotServer/commit/3bb9d65))
* **gbdialog:** Trying to save context. ([ce04290](https://github.com/pragmatismo-io/BotServer/commit/ce04290))
* **gbdialog:** Updating packages to latest versions and sync *-lock file. ([dcafb7a](https://github.com/pragmatismo-io/BotServer/commit/dcafb7a))
* **gbdialog:** VBA hear must be a wrapper call. ([6915d58](https://github.com/pragmatismo-io/BotServer/commit/6915d58))
* **gbdialog:** VBA is running financial simulations. ([9fb431c](https://github.com/pragmatismo-io/BotServer/commit/9fb431c))
* **gbdialog:** VBA is running. ([2dd359a](https://github.com/pragmatismo-io/BotServer/commit/2dd359a))
* **gbdialog:** VBA loop done - one thing left to automate: Hear wrapper. ([776fe03](https://github.com/pragmatismo-io/BotServer/commit/776fe03))
* **package:** update azure-arm-resource to version 7.2.1 ([4e72507](https://github.com/pragmatismo-io/BotServer/commit/4e72507))
* **package:** update botlib to version 0.1.7 ([8205599](https://github.com/pragmatismo-io/BotServer/commit/8205599))
* **package:** update csv-parse to version 4.0.0 ([3fb5a9a](https://github.com/pragmatismo-io/BotServer/commit/3fb5a9a))
* **package:** update marked to version 0.5.2 ([405fc96](https://github.com/pragmatismo-io/BotServer/commit/405fc96))
* **package:** update pragmatismo-io-framework to version 1.0.19 ([67c2ce7](https://github.com/pragmatismo-io/BotServer/commit/67c2ce7))
* **tests:** Disabling VM tests tentative for now. ([9d5a9c6](https://github.com/pragmatismo-io/BotServer/commit/9d5a9c6))


### Features

* **gbdialog:** The first VBA code is run. ([f0a0cd3](https://github.com/pragmatismo-io/BotServer/commit/f0a0cd3))
* **scripting:** First code changes to VBA implementation. ([09715bc](https://github.com/pragmatismo-io/BotServer/commit/09715bc))

## [1.0.8](https://github.com/pragmatismo-io/BotServer/compare/1.0.7...1.0.8) (2018-11-18)


### Bug Fixes

* **docs:** Video thumbnail update to raw picture URL. ([564b394](https://github.com/pragmatismo-io/BotServer/commit/564b394))

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
