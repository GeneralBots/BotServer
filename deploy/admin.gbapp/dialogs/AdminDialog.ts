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


import { UrlJoin } from "urljoin";
import { AzureSearch } from "pragmatismo-io-framework";
import { Prompts, Session, UniversalBot } from 'botbuilder';
import { GBMinInstance } from "botlib";
import { IGBDialog } from "botlib";
import { GBDeployer } from '../../core.gbapp/services/GBDeployer';
import { GBImporter } from '../../core.gbapp/services/GBImporter';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService';
import { KBService } from './../../kb.gbapp/services/KBService';

export class AdminDialog extends IGBDialog {

  static setup(bot: UniversalBot, min: GBMinInstance) {

    let importer = new GBImporter(min.core);
    let deployer = new GBDeployer(min.core, importer);

    bot
      .dialog("/admin", [
        (session: Session, args) => {
          Prompts.text(session, "Please, authenticate:");
          if (args == undefined || args.firstRun) {
          }
        },
        (session: Session, results) => {
          var text = results.response;
          if (
            !session.privateConversationData.authenticated ||
            text === GBConfigService.get("ADMIN_PASS")
          ) {
            session.privateConversationData.authenticated = true;
            session.send(
              "Welcome to Pragmatismo.io GeneralBots Administration."
            );
            Prompts.text(session, "Which task do you wanna run now?");
          } else {
            session.endDialog();
          }
        },
        function (session: Session, results) {
          var text = results.response;
          if (text === "quit") {
            session.privateConversationData.authenticated = false;
            session.replaceDialog("/");
          } else if (text === "sync") {
            min.core.syncDatabaseStructure(() => { });
            session.send("Sync started...");
            session.replaceDialog("/admin", {
              firstRun: false
            });
          } else if (text.split(" ")[0] === "rebuildIndex") {
            AdminDialog.rebuildIndexCommand(min, session, () =>
              session.replaceDialog("/admin", {
                firstRun: false
              })
            );
          } else if (text.split(" ")[0] === "deployPackage") {
            AdminDialog.deployPackageCommand(text, session, deployer, min, () =>
              session.replaceDialog("/admin", {
                firstRun: false
              })
            );
          } else if (text.split(" ")[0] === "redeployPackage") {
            AdminDialog.undeployPackageCommand(text, min, session, () => {
              AdminDialog.deployPackageCommand(text, session, deployer, min, () => {
                session.send("Redeploy done.");
                session.replaceDialog("/admin", {
                  firstRun: false
                });
              });
            });
          } else if (text.split(" ")[0] === "undeployPackage") {
            AdminDialog.undeployPackageCommand(text, min, session, () =>
              session.replaceDialog("/admin", {
                firstRun: false
              })
            );
          } else if (text.split(" ")[0] === "applyPackage") {
            session.send("Applying in progress...");
            min.core.loadInstance(text.split(" ")[1], (item, err) => {
              session.send("Applying done...");
              session.replaceDialog("/");
            });
            session.replaceDialog("/admin", {
              firstRun: false
            });
          }
        }
      ])
      .triggerAction({
        matches: /^(admin)/i
      });
  }

  static undeployPackageCommand(text: any, min: GBMinInstance, session: Session, cb) {
    let packageName = text.split(" ")[1];
    let importer = new GBImporter(min.core);
    let deployer = new GBDeployer(min.core, importer);
    session.send(`Undeploying package ${packageName}...`);
    deployer.undeployPackageFromLocalPath(
      min.instance,
      UrlJoin("deploy", packageName),
      (data, err) => {
        session.send(`Package ${packageName} undeployed...`);
        cb();
      }
    );
  }

  static deployPackageCommand(
    text: string,
    session: Session,
    deployer: GBDeployer,
    min: GBMinInstance,
    cb
  ) {
    let packageName = text.split(" ")[1];
    session.send(`Deploying package ${packageName}... (It may take a few seconds)`);

    // TODO: Find packages in all posible locations.
    let additionalPath = GBConfigService.get("ADDITIONAL_DEPLOY_PATH");

    deployer.deployPackageFromLocalPath(
      UrlJoin(additionalPath, packageName),
      (data, err) => {
        session.send(`Package ${packageName} deployed... Please run rebuildIndex command.`);

      }
    );
  }

  static rebuildIndexCommand(min: GBMinInstance, session: Session, cb) {
    let search = new AzureSearch(
      min.instance.searchKey,
      min.instance.searchHost,
      min.instance.searchIndex,
      min.instance.searchIndexer
    );
    session.send("Rebuilding index...");
    search.deleteIndex((data, err) => {
      let kbService = new KBService();
      search.createIndex(kbService.getSearchSchema(min.instance.searchIndex), "gb", (data, err) => {
        session.send("Index rebuilt.");
      });
    });
  }
}
