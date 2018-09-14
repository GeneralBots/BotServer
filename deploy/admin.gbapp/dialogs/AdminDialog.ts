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

"use strict"

const UrlJoin = require("url-join")
import { AzureSearch } from "pragmatismo-io-framework"
import { GBMinInstance } from "botlib"
import { IGBDialog } from "botlib"
import { GBDeployer } from '../../core.gbapp/services/GBDeployer'
import { GBImporter } from '../../core.gbapp/services/GBImporter'
import { GBConfigService } from '../../core.gbapp/services/GBConfigService'
import { KBService } from './../../kb.gbapp/services/KBService'
import { BotAdapter } from "botbuilder"
import {messages} from "./Strings"
import { reject } from "async"

/**
 * Dialogs for administration tasks.
 */
export class AdminDialog extends IGBDialog {


  static async undeployPackageCommand(text: any, min: GBMinInstance, dc) {
    let packageName = text.split(" ")[1]
    let importer = new GBImporter(min.core)
    let deployer = new GBDeployer(min.core, importer)
    dc.context.sendActivity(`Undeploying package ${packageName}...`)
    await deployer.undeployPackageFromLocalPath(
      min.instance,
      UrlJoin("deploy", packageName))
    dc.context.sendActivity(`Package ${packageName} undeployed...`)
  }

  static async deployPackageCommand(
    text: string,
    dc,
    deployer: GBDeployer,
    min: GBMinInstance
  ) {
    let packageName = text.split(" ")[1]
    await dc.context.sendActivity(`Deploying package ${packageName}... (It may take a few seconds)`)
    let additionalPath = GBConfigService.get("ADDITIONAL_DEPLOY_PATH")
    await deployer.deployPackageFromLocalPath(UrlJoin(additionalPath, packageName))
    await dc.context.sendActivity(`Package ${packageName} deployed... Please run rebuildIndex command.`)
  }

  static async rebuildIndexCommand(min: GBMinInstance, dc) {
    let search = new AzureSearch(
      min.instance.searchKey,
      min.instance.searchHost,
      min.instance.searchIndex,
      min.instance.searchIndexer
    )
    dc.context.sendActivity("Rebuilding index...")
    await search.deleteIndex()
    let kbService = new KBService(min.core.sequelize)
    await search.createIndex(kbService.getSearchSchema(min.instance.searchIndex), "gb")
    await dc.context.sendActivity("Index rebuilt.")
  }

  /**
   * Setup dialogs flows and define services call.
   * 
   * @param bot The bot adapter.
   * @param min The minimal bot instance data.
   */
  static setup(bot: BotAdapter, min: GBMinInstance) {

    // Setup services.

    let importer = new GBImporter(min.core)
    let deployer = new GBDeployer(min.core, importer)

    min.dialogs.add("/admin", [

      async (dc) => {

        await dc.context.sendActivity(`Deploying package ... (It may take a few seconds)`)
        await AdminDialog.deployPackageCommand("deployPackage ProjectOnline.gbkb", dc, deployer, min)
        await dc.endAll()

      }])

    min.dialogs.add("/admin1", [

      async (dc, args) => {
        const prompt = "Please, authenticate:"
        await dc.prompt('textPrompt', prompt)
      },
      async (dc, value) => {
        let text = value
        const user = min.userState.get(dc.context)

        if (
          !user.authenticated ||
          text === GBConfigService.get("ADMIN_PASS")
        ) {
          user.authenticated = true
          await dc.context.sendActivity(
            "Welcome to Pragmatismo.io GeneralBots Administration."
          )
          await dc.prompt('textPrompt', "Which task do you wanna run now?")
        } else {
          await dc.endAll()
        }
      },
      async (dc, value) => {
        var text = value
        const user = min.userState.get(dc.context)

        if (text === "quit") {
          user.authenticated = false
          await dc.replace("/")
        } else if (text === "sync") {
          await min.core.syncDatabaseStructure()
          await dc.context.sendActivity("Sync started...")
          await dc.replace("/admin", { firstRun: false })
        } else if (text.split(" ")[0] === "rebuildIndex") {
          await AdminDialog.rebuildIndexCommand(min, dc)
          await dc.replace("/admin", { firstRun: false })
        } else if (text.split(" ")[0] === "deployPackage") {
          await AdminDialog.deployPackageCommand(text, dc, deployer, min)
          await dc.replace("/admin", { firstRun: false })
        } else if (text.split(" ")[0] === "redeployPackage") {
          await AdminDialog.undeployPackageCommand(text, min, dc)
          await AdminDialog.deployPackageCommand(text, dc, deployer, min)
          await dc.context.sendActivity("Redeploy done.")
          await dc.replace("/admin", { firstRun: false })
        } else if (text.split(" ")[0] === "undeployPackage") {
          await AdminDialog.undeployPackageCommand(text, min, dc)
          await dc.replace("/admin", { firstRun: false })
        } else if (text.split(" ")[0] === "applyPackage") {
          await dc.context.sendActivity("Applying in progress...")
          await min.core.loadInstance(text.split(" ")[1])
          await dc.context.sendActivity("Applying done...")
          await dc.replace("/admin", { firstRun: false })
        } else if (text.split(" ")[0] === "rat") {
          await min.conversationalService.sendEvent(dc, "play", { playerType: "login", data: null })
          await dc.context.sendActivity("Realize login clicando no botão de login, por favor...")
        }
      }
    ])
  }
}
