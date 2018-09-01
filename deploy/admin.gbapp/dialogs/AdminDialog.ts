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

const UrlJoin = require("url-join");

import { AzureSearch } from "pragmatismo-io-framework";
const { DialogSet, TextPrompt, NumberPrompt } = require('botbuilder-dialogs');
const { createTextPrompt, createNumberPrompt } = require('botbuilder-prompts');
import { GBMinInstance } from "botlib";
import { IGBDialog } from "botlib";
import { GBDeployer } from '../../core.gbapp/services/GBDeployer';
import { GBImporter } from '../../core.gbapp/services/GBImporter';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService';
import { KBService } from './../../kb.gbapp/services/KBService';
import { BotAdapter } from "botbuilder";

export class AdminDialog extends IGBDialog {

  static setup(bot: BotAdapter, min: GBMinInstance) {

    let importer = new GBImporter(min.core);
    let deployer = new GBDeployer(min.core, importer);

    min.dialogs.add("/admin", [
        async (dc, args) => {
          const prompt = "Please, authenticate:";
          await dc.prompt('textPrompt', prompt);
        },
        async (dc, value) => {
          var text = value.response;
          const user = min.userState.get(dc.context);

          if (
            !user.authenticated ||
            text === GBConfigService.get("ADMIN_PASS")
          ) {
            user.authenticated = true;
            dc.context.sendActivity(
              "Welcome to Pragmatismo.io GeneralBots Administration."
            );
            await dc.prompt('textPrompt', "Which task do you wanna run now?");
          } else {
            dc.endAll();
          }
        },
        async (dc, value) => {
          var text = value;
          const user = min.userState.get(dc.context);

          if (text === "quit") {
            user.authenticated = false;
            dc.replace("/");
          } else if (text === "sync") {
            min.core.syncDatabaseStructure(() => { });
            dc.context.sendActivity("Sync started...");
            dc.replace("/admin", {
              firstRun: false
            });
          } else if (text.split(" ")[0] === "rebuildIndex") {
            AdminDialog.rebuildIndexCommand(min, dc, () =>
              dc.replace("/admin", {
                firstRun: false
              })
            );
          } else if (text.split(" ")[0] === "deployPackage") {
            AdminDialog.deployPackageCommand(text, dc, deployer, min, () =>
              dc.replace("/admin", {
                firstRun: false
              })
            );
          } else if (text.split(" ")[0] === "redeployPackage") {
            AdminDialog.undeployPackageCommand(text, min, dc, () => {
              AdminDialog.deployPackageCommand(text, dc, deployer, min, () => {
                dc.context.sendActivity("Redeploy done.");
                dc.replace("/admin", {
                  firstRun: false
                });
              });
            });
          } else if (text.split(" ")[0] === "undeployPackage") {
            AdminDialog.undeployPackageCommand(text, min, dc, () =>
              dc.replace("/admin", {
                firstRun: false
              })
            );
          } else if (text.split(" ")[0] === "applyPackage") {
            dc.context.sendActivity("Applying in progress...");
            min.core.loadInstance(text.split(" ")[1], (item, err) => {
              dc.context.sendActivity("Applying done...");
              dc.replace("/");
            });
            dc.replace("/admin", {
              firstRun: false
            });
          }
        }
      ])
  }

  static undeployPackageCommand(text: any, min: GBMinInstance, dc, cb) {
    let packageName = text.split(" ")[1];
    let importer = new GBImporter(min.core);
    let deployer = new GBDeployer(min.core, importer);
    dc.context.sendActivity(`Undeploying package ${packageName}...`);
    deployer.undeployPackageFromLocalPath(
      min.instance,
      UrlJoin("deploy", packageName),
      (data, err) => {
        dc.context.sendActivity(`Package ${packageName} undeployed...`);
        cb();
      }
    );
  }

  static deployPackageCommand(
    text: string,
    dc,
    deployer: GBDeployer,
    min: GBMinInstance,
    cb
  ) {
    let packageName = text.split(" ")[1];
    dc.context.sendActivity(`Deploying package ${packageName}... (It may take a few seconds)`);

    // TODO: Find packages in all possible locations.
    let additionalPath = GBConfigService.get("ADDITIONAL_DEPLOY_PATH");

    deployer.deployPackageFromLocalPath(
      UrlJoin(additionalPath, packageName),
      (data, err) => {
        dc.context.sendActivity(`Package ${packageName} deployed... Please run rebuildIndex command.`);

      }
    );
  }

  static rebuildIndexCommand(min: GBMinInstance, dc, cb) {
    let search = new AzureSearch(
      min.instance.searchKey,
      min.instance.searchHost,
      min.instance.searchIndex,
      min.instance.searchIndexer
    );
    dc.context.sendActivity("Rebuilding index...");
    search.deleteIndex((data, err) => {
      let kbService = new KBService();
      search.createIndex(kbService.getSearchSchema(min.instance.searchIndex), "gb", (data, err) => {
        dc.context.sendActivity("Index rebuilt.");
      });
    });
  }
}
