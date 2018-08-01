/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| ( ) |( (_) )  |
|   | ,__/'(_)  `\__,_)`\__  |(_) (_) (_)`\__,_)`\__)(_)(____/(_) (_)`\___/'  |
|   | |                ( )_) |                                                |
|   (_)                 \___/'                                                |
|                                                                             |
| General Bots Copyright (c) Pragmatismo.io. All rights reserved.             |
| Licensed under the AGPL-3.0.                                                |
|                                                                             | 
| According to our dual licensing model, this program can be used either      |
| under the terms of the GNU Affero General Public License, version 3,        |
| or under a proprietary license.                                             |
|                                                                             |
| The texts of the GNU Affero General Public License with an additional       |
| permission and of our proprietary license can be found at and               |
| in the LICENSE file you have received along with this program.              |
|                                                                             |
| This program is distributed in the hope that it will be useful,             |
| but WITHOUT ANY WARRANTY; without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of Pragmatismo.io.                 |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

"use strict";

const gBuilder = require("botbuilder");
const UrlJoin = require("url-join");
const Path = require("path");
const Fs = require("fs");
const Url = require("url");
const logger = require("../../../src/logger");
const WaitUntil = require("wait-until");
const Walk = require("fs-walk");
const express = require("express");

import { UniversalBot } from "botbuilder";
import { Session, MemoryBotStorage, Message } from "botbuilder";
import { GBCoreService } from "./GBCoreService";
import { GBConversationalService } from "./GBConversationalService";
import { GBConfigService } from "./GBConfigService";
import * as request from "request-promise-native";
import { GBMinInstance, IGBCoreService, IGBInstance, IGBPackage, GBError } from "botlib";
import { GBServiceCallback } from "botlib";
import { GBAnalyticsPackage } from "../../analytics.gblib";
import { GBCorePackage } from "../../core.gbapp";
import { GBKBPackage } from '../../kb.gbapp';
import { GBDeployer } from './GBDeployer';
import { GBSecurityPackage } from '../../security.gblib';
import { GBAdminPackage } from './../../admin.gbapp/index';
import { GBCustomerSatisfactionPackage } from "../../customer-satisfaction.gbapp";
import { GBWhatsappPackage } from "../../whatsapp.gblib";

/** Minimal service layer for a bot. */

export class GBMinService {

  core: GBCoreService;
  conversationalService: GBConversationalService;
  deployer: GBDeployer;

  deployFolder = "deploy";
  corePackage = "core.gbai";

  /**
   * Static iniatialization of minimal instance.
   *
   * @param core Basic database services to identify instance, for example.
   * @param cb Returns the loaded instance.
   */
  constructor(
    core: GBCoreService,
    conversationalService: GBConversationalService,
    deployer: GBDeployer
  ) {
    this.core = core;
    this.conversationalService = conversationalService;
    this.deployer = deployer;
  }

  /** Constructs a new minimal instance for each bot. */

  buildMin(cb: GBServiceCallback<GBMinInstance>, server: any, appPackages: Array<IGBPackage>, botPackages: Array<string>) {

    var _this_ = this;

    // Serves default UI on root address '/'.

    let uiPackage = "default.gbui";
    server.use(
      "/",
      express.static(UrlJoin(this.deployFolder, uiPackage, "build"))
    );

    // Loads all bot instances from storage. 

    _this_.core.loadInstances((instances: IGBInstance[], err) => {

      // We are running empty !

      if (!instances) {
        logger.error('The bot server is running empty.');
        return;
      }

      // Gets the authorization key for each instance from Bot Service.

      instances.forEach(instance => {
        let options = {
          url:
            "https://directline.botframework.com/v3/directline/tokens/generate",
          method: "POST",
          headers: {
            Authorization: `Bearer ${instance.webchatKey}`
          }
        };
        request(options).then((response:
          string) => {

          // Serves the bot information object via http so clients can get
          // instance information stored on server.

          let responseObject = JSON.parse(response);
          server.get("/instances/:botId", (req, res) => {

            // Returns the instance object to clients requesting bot info.

            let botId = req.params.botId;
            _this_.core.loadInstance(
              botId,
              (instance: IGBInstance, err) => {
                if (instance) {

                  let options = {
                    url:
                      "https://westus.api.cognitive.microsoft.com/sts/v1.0/issueToken",
                    method: "POST",
                    headers: {
                      "Ocp-Apim-Subscription-Key": instance.speechKey
                    }
                  };
                  request(options).then((response:
                    string) => {

                    res.send(
                      JSON.stringify({
                        instanceId: instance.instanceId,
                        botId: botId,
                        theme: instance.theme,
                        secret: instance.webchatKey, // TODO: Use token.
                        speechToken: response,
                        conversationId: responseObject.conversationId
                      })
                    );
                  }).catch(err => {
                    logger.trace(`Error loading sts service: ${err}.`);
                  });
                } else {
                  let error = `Instance not found: ${botId}.`;
                  res.send(error);
                  logger.error(error);
                }
              }
            );
          });
        });

        // The minimal bot is built here.

        let min = new GBMinInstance();
        min.botId = instance.botId;
        min.core = _this_.core;
        min.conversationalService = _this_.conversationalService;
        _this_.core.loadInstance(min.botId, (data, err) => {

          min.instance = data;

          // Call the loadBot event for all packages.

          appPackages.forEach(e => {
            e.sysPackages = new Array<IGBPackage>();
            [GBAdminPackage, GBAnalyticsPackage, GBCorePackage, GBSecurityPackage,
              GBKBPackage, GBCustomerSatisfactionPackage, GBWhatsappPackage].forEach(sysPackage => {
                logger.trace(`Loading sys package: ${sysPackage.name}...`);
                let p = Object.create(sysPackage.prototype) as IGBPackage;
                p.loadBot(min);
                e.sysPackages.push(p);
                if (sysPackage.name === "GBWhatsappPackage") {
                  let url = "/instances/:botId/whatsapp";
                  server.post(url, (req, res) => {
                    p["channel"].received(req, res);
                  });
                }
              });
          });

          botPackages.forEach(e => {
            [GBAdminPackage, GBAnalyticsPackage, GBCorePackage, GBSecurityPackage,
              GBKBPackage, GBCustomerSatisfactionPackage, GBWhatsappPackage].forEach(sysPackage => {
                logger.trace(`Loading sys package: ${sysPackage.name}...`);
                let p = Object.create(sysPackage.prototype) as IGBPackage;
                p.loadBot(min);
              });
          });
        });

        let connector = new gBuilder.ChatConnector({
          appId: instance.marketplaceId,
          appPassword: instance.marketplacePassword
        });

        // Serves individual URL for each bot conversational interface...

        let url = `/api/messages/${instance.botId}`;
        logger.trace(
          `GeneralBots(${instance.engineName}) listening on: ${url}.`
        );
        server.post(url, connector.listen());

        // Serves individual URL for each bot user interface.

        let uiUrl = `/${instance.botId}`;
        server.use(
          uiUrl,
          express.static(UrlJoin(this.deployFolder, uiPackage, "build"))
        );
        logger.trace(`Bot UI ${uiPackage} acessible at: ${uiUrl}.`);


        // Prepares bot service.

        let inMemoryStorage = new MemoryBotStorage();

        min.bot = new gBuilder.UniversalBot(connector, {
          storage: inMemoryStorage
        });


        // Setups handlers.

        min.bot.use({

          botbuilder: (session, next) => {

            if (!session.privateConversationData.loaded) {
              setTimeout(
                () => {
                  min.conversationalService.sendEvent(
                    session,
                    "loadInstance",
                    min.instance // TODO: Send a new thiner object.
                  )
                },
                500
              );
              session.privateConversationData.loaded = true;
              session.userData.subjects = [];
            }

            appPackages.forEach(e => {
              e.onNewSession(min, session);
            });

            next();
          },
          receive: function (event: any, next) {
            logger.trace(
              `[RCV]: ChannelID: ${event.address.channelId}, ConversationID: ${event.address.conversation.id}
               Type: ${event.type}, Name: ${event.name}, Text: ${event.text}.`
            );

            // PACKAGE: Provide loop here.

            if (
              event.type === "conversationUpdate" &&
              event.membersAdded.length > 0
              // TODO: Is it really Necessary? !event.membersAdded[0].id.startsWith('general-bot-9672a8d3') //DEMO: min.botId) //TODO: Check entire collection.
            ) {

              if (event.address.channelId != "directline") {
                min.bot.beginDialog(event.address, "/");
              }
              else {
                next();
              }
            } else if (event.name === "whoAmI") {
              min.bot.beginDialog(event.address, "/whoAmI");
            } else if (event.name === "showSubjects") {
              min.bot.beginDialog(event.address, "/menu");
            } else if (event.name === "giveFeedback") {
              min.bot.beginDialog(event.address, "/feedback", {
                fromMenu: true
              });
            } else if (event.name === "showFAQ") {
              min.bot.beginDialog(event.address, "/faq");
            } else if (event.name === "ask") {
              min.bot.beginDialog(event.address, "/answer", {
                query: event.data,
                fromFaq: true
              });
            } else if (event.name === "quality") {
              min.bot.beginDialog(event.address, "/quality", {
                score: event.data
              });
            } else {
              next();
            }
          },
          send: function (event, next) {
            logger.trace(
              `[SND]: ChannelID: ${event.address.channelId}, ConversationID: ${event.address.conversation},
               Type: ${event.type}              `);
            this.core.createMessage(
              this.min.conversation,
              this.min.conversation.startedBy,
              event.source,
              (data, err) => {
                logger.trace(event.source);
              }
            );
            next();
          }
        });


        // Specialized load for each min instance.

        cb(min, null);
      });
    });
  }

  /** Performs package deployment in all .gbai or default. */
  public deployPackages(core: IGBCoreService, server: any, appPackages: Array<IGBPackage>, botPackages: Array<string>) {

    return new Promise((resolve, reject) => {
      try {
        var _this_ = this;
        let totalPackages = 0;
        let additionalPath = GBConfigService.get("ADDITIONAL_DEPLOY_PATH");
        let paths = [this.deployFolder];
        if (additionalPath) {
          paths = paths.concat(additionalPath.toLowerCase().split(";"));
        }
        let gbappPackages = new Array<string>();
        let generalPackages = new Array<string>();

        function doIt(path) {
          const isDirectory = source => Fs.lstatSync(source).isDirectory()
          const getDirectories = source =>
            Fs.readdirSync(source).map(name => Path.join(source, name)).filter(isDirectory)

          let dirs = getDirectories(path);
          dirs.forEach(element => {
            if (element.startsWith('.')) {
              logger.trace(`Ignoring ${element}...`);
            } else {
              if (element.endsWith('.gbot')) {
                botPackages.push(element);
              } else if (element.endsWith('.gbapp')) {
                gbappPackages.push(element);
              } else {
                generalPackages.push(element);
              }
            }
          });
        }

        logger.trace(`Starting looking for generalPackages...`);
        paths.forEach(e => {
          logger.trace(`Looking in: ${e}...`);
          doIt(e);
        });

        /** Deploys all .gbapp files first. */

        let appPackagesProcessed = 0;

        gbappPackages.forEach(e => {
          logger.trace(`Deploying app: ${e}...`);
          // Skips .gbapp inside deploy folder.
          if (!e.startsWith('deploy')) {
            import(e).then(m => {
              let p = new m.Package() as IGBPackage;
              p.loadPackage(core, core.sequelize);
              appPackages.push(p);
              logger.trace(`App (.gbapp) deployed: ${e}.`);
              appPackagesProcessed++;
            }, err => {
              logger.trace(`Error deploying App (.gbapp): ${e}: ${err}`);
              appPackagesProcessed++;
            }).catch(err => {
              logger.trace(`Error deploying App (.gbapp): ${e}: ${err}`);
              appPackagesProcessed++;
            });
          } else {
            appPackagesProcessed++;
          }
        });

        WaitUntil()
          .interval(1000)
          .times(10)
          .condition(function (cb) {
            logger.trace(`Waiting for app package deployment...`);
            cb(appPackagesProcessed == gbappPackages.length);
          })
          .done(function (result) {
            logger.trace(`App Package deployment done.`);

            core.syncDatabaseStructure(cb => {

              /** Deploys all .gbot files first. */

              botPackages.forEach(e => {
                logger.trace(`Deploying bot: ${e}...`);
                _this_.deployer.deployBot(e, (data, err) => {
                  logger.trace(`Bot: ${e} deployed...`);
                });
              });

              // TODO: Wait here.

              /** Then all remaining generalPackages are loaded. */

              generalPackages.forEach(filename => {

                let filenameOnly = Path.basename(filename);
                logger.trace(`Deploying package: ${filename}...`);

                /** Handles apps for general bots - .gbapp must stay out of deploy folder. */

                if (Path.extname(filename) === ".gbapp" || Path.extname(filename) === ".gblib") {


                  /** Themes for bots. */

                } else if (Path.extname(filename) === ".gbtheme") {
                  server.use("/themes/" + filenameOnly, express.static(filename));
                  logger.trace(`Theme (.gbtheme) assets acessible at: ${"/themes/" + filenameOnly}.`);


                  /** Knowledge base for bots. */

                } else if (Path.extname(filename) === ".gbkb") {
                  server.use(
                    "/kb/" + filenameOnly + "/subjects",
                    express.static(UrlJoin(filename, "subjects"))
                  );
                  logger.trace(`KB (.gbkb) assets acessible at: ${"/kb/" + filenameOnly}.`);
                }

                else if (Path.extname(filename) === ".gbui" || filename.endsWith(".git")) {
                  // Already Handled
                }

                /** Unknown package format. */

                else {
                  let err = new Error(`Package type not handled: ${filename}.`);
                  reject(err);
                }
                totalPackages++;
              });

              WaitUntil()
                .interval(1000)
                .times(5)
                .condition(function (cb) {
                  logger.trace(`Waiting for package deployment...`);
                  cb(totalPackages == (generalPackages.length));
                })
                .done(function (result) {
                  if (botPackages.length === 0) {
                    logger.info(`The bot server is running empty: No bot instances have been found, at least one .gbot file must be deployed.`);
                  }
                  else {
                    logger.trace(`Package deployment done.`);
                  }
                  resolve();
                });
            });
          });
      } catch (err) {
        logger.error(err);
        reject(err)
      }
    });
  }
}