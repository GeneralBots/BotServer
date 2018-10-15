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
| but WITHOUT ANY WARRANTY without even the implied warranty of               |
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
import { GBMinInstance } from "botlib";
import { IGBDialog } from "botlib";
import { GBDeployer } from "../../core.gbapp/services/GBDeployer";
import { GBImporter } from "../../core.gbapp/services/GBImporter";
import { GBConfigService } from "../../core.gbapp/services/GBConfigService";
import { BotAdapter } from "botbuilder";
import { GBAdminService } from "../services/GBAdminService";
import { Messages } from "../strings";


/**
 * Dialogs for administration tasks.
 */
export class AdminDialog extends IGBDialog {

  static async createFarmCommand(text: any, min: GBMinInstance) {
  }

  static async undeployPackageCommand(text: any, min: GBMinInstance) {
    let packageName = text.split(" ")[1];
    let importer = new GBImporter(min.core);
    let deployer = new GBDeployer(min.core, importer);
    await deployer.undeployPackageFromLocalPath(
      min.instance,
      UrlJoin("deploy", packageName)
    );
  }

  static async deployPackageCommand(text: string, deployer: GBDeployer) {
    let packageName = text.split(" ")[1];
    let additionalPath = GBConfigService.get("ADDITIONAL_DEPLOY_PATH");
    await deployer.deployPackageFromLocalPath(
      UrlJoin(additionalPath, packageName)
    );
  }
  /**
   * Setup dialogs flows and define services call.
   *
   * @param bot The bot adapter.
   * @param min The minimal bot instance data.
   */
  static setup(bot: BotAdapter, min: GBMinInstance) {
    // Setup services.

    let importer = new GBImporter(min.core);
    let deployer = new GBDeployer(min.core, importer);

    min.dialogs.add("/admin", [
      async dc => {
        const locale = dc.context.activity.locale;
        const prompt = Messages[locale].authenticate;
        await dc.prompt("textPrompt", prompt);
      },
      async (dc, password) => {
        const locale = dc.context.activity.locale;
        if (
          password === GBConfigService.get("ADMIN_PASS") &&
          GBAdminService.StrongRegex.test(password)
        ) {
          await dc.context.sendActivity(Messages[locale].welcome);
          await dc.prompt("textPrompt", Messages[locale].which_task);
        } else {
          await dc.prompt("textPrompt", Messages[locale].wrong_password);
          await dc.endAll();
        }
      },
      async (dc, value) => {
        const locale = dc.context.activity.locale;
        var text = value;
        let cmdName = text.split(" ")[0];

        dc.context.sendActivity(Messages[locale].working(cmdName));
        let unknownCommand = false;

        if (text === "quit") {
          await dc.replace("/");
        } else if (cmdName === "createFarm") {
          await AdminDialog.createFarmCommand(text, deployer);
          await dc.replace("/admin", { firstRun: false });
        } else if (cmdName === "deployPackage") {
          await AdminDialog.deployPackageCommand(text, deployer);
          await dc.replace("/admin", { firstRun: false });
        } else if (cmdName === "redeployPackage") {
          await AdminDialog.undeployPackageCommand(text, min);
          await AdminDialog.deployPackageCommand(text, deployer);
          await dc.context.sendActivity();
          await dc.replace("/admin", { firstRun: false });
        } else if (cmdName === "undeployPackage") {
          await AdminDialog.undeployPackageCommand(text, min);
          await dc.replace("/admin", { firstRun: false });
        } else if (cmdName === "setupSecurity") {
          await AdminDialog.setupSecurity(min, dc);
        } else {
          unknownCommand = true;
        }

        if (unknownCommand) {
          await dc.context.sendActivity(Messages[locale].unknown_command);
        } else {
          await dc.context.sendActivity(
            Messages[locale].finshed_working(cmdName)
          );
        }
        await dc.endAll();
        await dc.replace("/answer", { query: text });
      }
    ]);
  }

  private static async setupSecurity(min: any, dc: any) {
    const locale = dc.context.activity.locale;
    let state = `${min.instance.instanceId}${Math.floor(
      Math.random() * 1000000000
    )}`;
    await min.adminService.setValue(
      min.instance.instanceId,
      "AntiCSRFAttackState",
      state
    );
    let url = `https://login.microsoftonline.com/${
      min.instance.authenticatorTenant
    }/oauth2/authorize?client_id=${
      min.instance.authenticatorClientId
    }&response_type=code&redirect_uri=${min.instance.botEndpoint}/${
      min.instance.botId
    }/token&state=${state}&response_mode=query`;

    await dc.context.sendActivity(Messages[locale].consent(url));
  }
}
