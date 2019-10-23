## [1.7.5](https://github.com/pragmatismo-io/BotServer/compare/1.7.4...1.7.5) (2019-10-18)


### Bug Fixes

* **core.gbapp:** Clean-up of system code ([9311943](https://github.com/pragmatismo-io/BotServer/commit/9311943))

## [1.7.4](https://github.com/pragmatismo-io/BotServer/compare/1.7.3...1.7.4) (2019-10-17)


### Bug Fixes

* **kb.gbapp:** Use of await style call ([a034abf](https://github.com/pragmatismo-io/BotServer/commit/a034abf))

## [1.7.3](https://github.com/pragmatismo-io/BotServer/compare/1.7.2...1.7.3) (2019-10-10)


### Bug Fixes

* **basic:** Adicionando .env no .gitignore e desabilitando suporte a linguagem BASIC por default ([91d1476](https://github.com/pragmatismo-io/BotServer/commit/91d1476))

## [1.7.2](https://github.com/pragmatismo-io/BotServer/compare/1.7.1...1.7.2) (2019-08-30)


### Bug Fixes

* **basic:** Fix default bot.vbs missing parenthesis in code. ([8501002](https://github.com/pragmatismo-io/BotServer/commit/8501002))

## [1.7.1](https://github.com/pragmatismo-io/BotServer/compare/1.7.0...1.7.1) (2019-08-30)


### Bug Fixes

* **whatsapp.gblib:** Multi-turn dialog fixed in BASIC processing. ([4689bfb](https://github.com/pragmatismo-io/BotServer/commit/4689bfb))

# [1.7.0](https://github.com/pragmatismo-io/BotServer/compare/1.6.0...1.7.0) (2019-08-29)


### Bug Fixes

* **whatsapp.gblib:** BASIC enabled with Promises. ([47df1f1](https://github.com/pragmatismo-io/BotServer/commit/47df1f1))
* **whatsapp.gblib:** BASIC is disabled again. ([fa9f260](https://github.com/pragmatismo-io/BotServer/commit/fa9f260))
* **whatsapp.gblib:** BASIC is disabled again. ([4828a33](https://github.com/pragmatismo-io/BotServer/commit/4828a33))
* **whatsapp.gblib:** User can say the activation code as the first message. ([3f6668d](https://github.com/pragmatismo-io/BotServer/commit/3f6668d))
* **whatsapp.gblib:** Web can show images again and working directory on startup being created. ([8d512ca](https://github.com/pragmatismo-io/BotServer/commit/8d512ca))
* **whatsapp.gblib:** Work directory being created now on startup. ([ca98965](https://github.com/pragmatismo-io/BotServer/commit/ca98965))


### Features

* **core.gbapp:** New bot management (CRUD) from SharePoint packages. ([9a961e7](https://github.com/pragmatismo-io/BotServer/commit/9a961e7))
* **kb.gbapp:** TSV are replaced with MS Excel files, to store tabular information. ([246b222](https://github.com/pragmatismo-io/BotServer/commit/246b222))
* **sharepoint:** SharePoint deployPackage added. ([ae73cf8](https://github.com/pragmatismo-io/BotServer/commit/ae73cf8))
* **whatsapp.gblib:** Image will be send as a message and md can be read easily on the phone. ([2b4fb68](https://github.com/pragmatismo-io/BotServer/commit/2b4fb68))
* **whatsapp.gblib:** Now Whatsapp will display markdown from .gbkb including images. ([faa5ec7](https://github.com/pragmatismo-io/BotServer/commit/faa5ec7))
* **whatsapp.gblib:** Same chat-api provider now shared between instances and deploy improvements. ([b2da413](https://github.com/pragmatismo-io/BotServer/commit/b2da413))
* **whatsapp.gblib:** Switch from Whatsapp channel which bot to talk to with the same number. ([650779e](https://github.com/pragmatismo-io/BotServer/commit/650779e))
* **whatsapp.gblib:** Switch from Whatsapp channel which bot to talk to with the same number. ([cb3d241](https://github.com/pragmatismo-io/BotServer/commit/cb3d241))

# [1.6.0](https://github.com/pragmatismo-io/BotServer/compare/1.5.5...1.6.0) (2019-08-21)


### Features

* **boot:** Bot now can be run with VSCode F5 after cloning from git. ([29d90db](https://github.com/pragmatismo-io/BotServer/commit/29d90db))

## [1.5.5](https://github.com/pragmatismo-io/BotServer/compare/1.5.4...1.5.5) (2019-07-23)


### Bug Fixes

* **kb.gbapp:** Improvement on translate ([bc56a1d](https://github.com/pragmatismo-io/BotServer/commit/bc56a1d))

## [1.5.4](https://github.com/pragmatismo-io/BotServer/compare/1.5.3...1.5.4) (2019-07-19)


### Bug Fixes

* **kb.gbapp:** NLP scoring added again. ([c83a5f9](https://github.com/pragmatismo-io/BotServer/commit/c83a5f9))

## [1.5.3](https://github.com/pragmatismo-io/BotServer/compare/1.5.2...1.5.3) (2019-07-18)


### Bug Fixes

* **core.gbapp:** Deployer now imports UTF-8 .tsv files. ([daf0741](https://github.com/pragmatismo-io/BotServer/commit/daf0741))

## [1.5.2](https://github.com/pragmatismo-io/BotServer/compare/1.5.1...1.5.2) (2019-07-18)


### Bug Fixes

* **core.gbapp:** Redeploy command now fixed. ([d68da40](https://github.com/pragmatismo-io/BotServer/commit/d68da40))

## [1.5.1](https://github.com/pragmatismo-io/BotServer/compare/1.5.0...1.5.1) (2019-07-18)


### Bug Fixes

* **kb.gbapp:** Answers in text in case of Whatsapp channel. ([4f994b1](https://github.com/pragmatismo-io/BotServer/commit/4f994b1))

# [1.5.0](https://github.com/pragmatismo-io/BotServer/compare/1.4.0...1.5.0) (2019-07-04)


### Features

* **core.gbapp:** New global quit keywords. ([15cd8d6](https://github.com/pragmatismo-io/BotServer/commit/15cd8d6))

# [1.4.0](https://github.com/pragmatismo-io/BotServer/compare/1.3.10...1.4.0) (2019-06-28)


### Bug Fixes

* **whatsapp.gblib:** Service latency due to res.end missing call. ([82dcfac](https://github.com/pragmatismo-io/BotServer/commit/82dcfac))


### Features

* **core.gbapp:** Development options added (ngrok) ([223801d](https://github.com/pragmatismo-io/BotServer/commit/223801d))
* **security.gblib:** Phone field and conversation refeerence. ([46261d7](https://github.com/pragmatismo-io/BotServer/commit/46261d7))

## [1.3.10](https://github.com/pragmatismo-io/BotServer/compare/1.3.9...1.3.10) (2019-06-21)


### Bug Fixes

* **core.gbapp:** MSFT changed again LUIS url. ([967f780](https://github.com/pragmatismo-io/BotServer/commit/967f780))

## [1.3.9](https://github.com/pragmatismo-io/BotServer/compare/1.3.8...1.3.9) (2019-06-18)


### Bug Fixes

* **whastapp.gblib:** Fix in pro-active messaging. ([3f710e8](https://github.com/pragmatismo-io/BotServer/commit/3f710e8))

## [1.3.8](https://github.com/pragmatismo-io/BotServer/compare/1.3.7...1.3.8) (2019-06-18)


### Bug Fixes

* **core.gbapp:** ms-rest-azure updated due to  https://github.com/Azure/ms-rest-js/issues/347. ([d61d2f8](https://github.com/pragmatismo-io/BotServer/commit/d61d2f8))
* **whatsapp.gblib:** Fixing loop behaviour. ([10d2a4a](https://github.com/pragmatismo-io/BotServer/commit/10d2a4a))

## [1.3.7](https://github.com/pragmatismo-io/BotServer/compare/1.3.6...1.3.7) (2019-06-05)


### Bug Fixes

* **core.gbapp:** Self-replication on Azure ([f29c8c2](https://github.com/pragmatismo-io/BotServer/commit/f29c8c2))
* **core.gbapp:** Self-replication on Azure ([4d484d0](https://github.com/pragmatismo-io/BotServer/commit/4d484d0))

## [1.3.6](https://github.com/pragmatismo-io/BotServer/compare/1.3.5...1.3.6) (2019-05-27)


### Bug Fixes

* **core.gbapp:** Self-replication on Azure ([5f0fb3b](https://github.com/pragmatismo-io/BotServer/commit/5f0fb3b))
* **core.gbapp:** Self-replication on Azure ([2509157](https://github.com/pragmatismo-io/BotServer/commit/2509157))
* **core.gbapp:** Self-replication on Azure ([8850370](https://github.com/pragmatismo-io/BotServer/commit/8850370))

## [1.3.5](https://github.com/pragmatismo-io/BotServer/compare/1.3.4...1.3.5) (2019-05-27)


### Bug Fixes

* **core.gbapp:** Self-replication on Azure ([f64cc4c](https://github.com/pragmatismo-io/BotServer/commit/f64cc4c))

## [1.3.4](https://github.com/pragmatismo-io/BotServer/compare/1.3.3...1.3.4) (2019-05-27)


### Bug Fixes

* **core.gbapp:** Self-replication on Azure ([e82a813](https://github.com/pragmatismo-io/BotServer/commit/e82a813))

## [1.3.3](https://github.com/pragmatismo-io/BotServer/compare/1.3.2...1.3.3) (2019-05-27)


### Bug Fixes

* **core.gbapp:** Self-replication on Azure ([bf602c6](https://github.com/pragmatismo-io/BotServer/commit/bf602c6))
* **core.gbapp:** Self-replication on Azure ([3cca504](https://github.com/pragmatismo-io/BotServer/commit/3cca504))
* **core.gbapp:** Self-replication on Azure ([05edafd](https://github.com/pragmatismo-io/BotServer/commit/05edafd))

## [1.3.2](https://github.com/pragmatismo-io/BotServer/compare/1.3.1...1.3.2) (2019-05-26)


### Bug Fixes

* **core.gbapp:** Self-replication on Azure. ([2ccae38](https://github.com/pragmatismo-io/BotServer/commit/2ccae38))
* **core.gbapp:** Self-replication on Azure. ([4b7d29d](https://github.com/pragmatismo-io/BotServer/commit/4b7d29d))

## [1.3.1](https://github.com/pragmatismo-io/BotServer/compare/1.3.0...1.3.1) (2019-05-25)


### Bug Fixes

* **core.gbapp:** Azure deployment. ([f1b8eb2](https://github.com/pragmatismo-io/BotServer/commit/f1b8eb2))
* **core.gbapp:** Azure deployment. ([49e1743](https://github.com/pragmatismo-io/BotServer/commit/49e1743))
* **core.gbapp:** Azure Deployment. ([f8fab38](https://github.com/pragmatismo-io/BotServer/commit/f8fab38))
* **core.gbapp:** ESNext in tsconfig.json to match MSFT. ([01b8dd5](https://github.com/pragmatismo-io/BotServer/commit/01b8dd5))
* **core.gbapp:** Fixing loading of instances. ([bc9c588](https://github.com/pragmatismo-io/BotServer/commit/bc9c588))
* **core.gbapp:** Investigating BASIC broken. ([f0ec25e](https://github.com/pragmatismo-io/BotServer/commit/f0ec25e))
* **core.gbapp:** package.json artifacts sync. ([01d34a6](https://github.com/pragmatismo-io/BotServer/commit/01d34a6))
* **core.gbapp:** Publishing in Azure. ([4d6779e](https://github.com/pragmatismo-io/BotServer/commit/4d6779e))
* **core.gbapp:** Removing POC code. ([56f46f4](https://github.com/pragmatismo-io/BotServer/commit/56f46f4))
* **kb.gbapp:** Typo fix. ([ba26578](https://github.com/pragmatismo-io/BotServer/commit/ba26578))
* **whatsapp.gblib:** Enabling Whatsapp. ([4351b87](https://github.com/pragmatismo-io/BotServer/commit/4351b87))
* **whatsapp.gblib:** Enabling Whatsapp. ([74f5936](https://github.com/pragmatismo-io/BotServer/commit/74f5936))
* **whatsapp.lib:** Enabling Whatsapp. ([85249e5](https://github.com/pragmatismo-io/BotServer/commit/85249e5))

# [1.3.0](https://github.com/pragmatismo-io/BotServer/compare/1.2.2...1.3.0) (2019-05-12)


### Bug Fixes

* **core:** ngrok is running on linux ([000bdc1](https://github.com/pragmatismo-io/BotServer/commit/000bdc1))
* **design:** 404 on bot logo on default.gbtheme: https://github.com/GeneralBots/BotServer/issues/80. ([f67f04a](https://github.com/pragmatismo-io/BotServer/commit/f67f04a))
* **gbot:** gbot.cmd now installs packages and compiles the server before running. ([dca0325](https://github.com/pragmatismo-io/BotServer/commit/dca0325))
* **gbot:** gbot.cmd now installs packages and compiles the server before running. ([b7abf5f](https://github.com/pragmatismo-io/BotServer/commit/b7abf5f))
* **general:** tslint being applied in all sources. ([77ccc3d](https://github.com/pragmatismo-io/BotServer/commit/77ccc3d))
* **general:** tslint being applied in all sources. ([25d1459](https://github.com/pragmatismo-io/BotServer/commit/25d1459))
* **general:** tslint being applied in all sources. ([4b49686](https://github.com/pragmatismo-io/BotServer/commit/4b49686))
* **general:** tslint being applied in all sources. ([895be68](https://github.com/pragmatismo-io/BotServer/commit/895be68))
* **general:** tslint being applied in all sources. ([c74b3ee](https://github.com/pragmatismo-io/BotServer/commit/c74b3ee))
* **general:** tslint being applied in all sources. ([ef3c5a1](https://github.com/pragmatismo-io/BotServer/commit/ef3c5a1))
* **general:** tslint being applied in all sources. ([e9bed77](https://github.com/pragmatismo-io/BotServer/commit/e9bed77))
* **general:** tslint being applied in all sources. ([d717de6](https://github.com/pragmatismo-io/BotServer/commit/d717de6))
* **general:** tslint being applied in all sources. ([2c18517](https://github.com/pragmatismo-io/BotServer/commit/2c18517))
* **general:** tslint being applied in all sources. ([cd5189d](https://github.com/pragmatismo-io/BotServer/commit/cd5189d))
* **general:** tslint being applied in all sources. ([5d08457](https://github.com/pragmatismo-io/BotServer/commit/5d08457))
* **general:** tslint being applied in all sources. ([6de285e](https://github.com/pragmatismo-io/BotServer/commit/6de285e))
* **general:** tslint being applied in all sources. ([69ca62b](https://github.com/pragmatismo-io/BotServer/commit/69ca62b))
* **general:** tslint being applied in all sources. ([8fec26c](https://github.com/pragmatismo-io/BotServer/commit/8fec26c))
* **kb.gbapp:** FAQ now showing again. ([c70200a](https://github.com/pragmatismo-io/BotServer/commit/c70200a))
* **kb.gbapp:** Fix in subjects null pointer. ([e21916f](https://github.com/pragmatismo-io/BotServer/commit/e21916f))
* **NLP:** Update of platform to mach NLP URL updates and versioning. ([6588049](https://github.com/pragmatismo-io/BotServer/commit/6588049))
* **VBA:** Several bugs fixed and refactoring on Deployer Service done. ([fecbd3e](https://github.com/pragmatismo-io/BotServer/commit/fecbd3e))


### Features

* **basic:** General Bots BASIC 2.0 with new keywords and parenthesis only when needed. ([3cc92ec](https://github.com/pragmatismo-io/BotServer/commit/3cc92ec))

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
