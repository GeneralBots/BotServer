#! /usr/bin / env node
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
| but WITHOUT ANY WARRANTY, without even the implied warranty of              |
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
const logger = require("./logger")
const express = require("express")
const bodyParser = require("body-parser")

import { Sequelize } from "sequelize-typescript"
import { GBConfigService } from "../deploy/core.gbapp/services/GBConfigService"
import { GBConversationalService } from "../deploy/core.gbapp/services/GBConversationalService"
import { GBMinService } from "../deploy/core.gbapp/services/GBMinService"
import { GBDeployer } from "../deploy/core.gbapp/services/GBDeployer"
import { GBWhatsappPackage } from './../deploy/whatsapp.gblib/index'
import { GBCoreService } from "../deploy/core.gbapp/services/GBCoreService"
import { GBImporter } from "../deploy/core.gbapp/services/GBImporter"
import { GBAnalyticsPackage } from "../deploy/analytics.gblib"
import { GBCorePackage } from "../deploy/core.gbapp"
import { GBKBPackage } from '../deploy/kb.gbapp'
import { GBSecurityPackage } from '../deploy/security.gblib'
import { GBAdminPackage } from '../deploy/admin.gbapp/index'
import { GBCustomerSatisfactionPackage } from "../deploy/customer-satisfaction.gbapp"
import { IGBPackage } from 'botlib'

let appPackages = new Array<IGBPackage>()

/**
 * General Bots open-core entry point.
 */
export class GBServer {

  /** Program entry-point. */
  static run() {

    // Creates a basic HTTP server that will serve several URL, one for each
    // bot instance. This allows the same server to attend multiple Bot on
    // the Marketplace until GB get serverless.

    let port = process.env.port || process.env.PORT || 4242
    logger.info(`The Bot Server is in STARTING mode...`)
    let server = express()

    server.use(bodyParser.json())       // to support JSON-encoded bodies
    server.use(bodyParser.urlencoded({     // to support URL-encoded bodies
      extended: true
    }))

    server.listen(port, () => {

      (async () => {
        try {

          logger.info(`Accepting connections on ${port}...`)
          logger.info(`Starting instances...`)

          // Reads basic configuration, initialize minimal services.

          GBConfigService.init()
          let core = new GBCoreService()
          await core.initDatabase()

          // Boot a bot package if any.

          let deployer = new GBDeployer(core, new GBImporter(core))

          // Build a minimal bot instance for each .gbot deployment.

          let conversationalService = new GBConversationalService(core)
          let minService = new GBMinService(core, conversationalService, deployer);

          [GBAdminPackage, GBAnalyticsPackage, GBCorePackage, GBSecurityPackage,
            GBKBPackage, GBCustomerSatisfactionPackage, GBWhatsappPackage].forEach(e => {
              logger.info(`Loading sys package: ${e.name}...`)
              let p = Object.create(e.prototype) as IGBPackage
              p.loadPackage(core, core.sequelize)
            })

          await deployer.deployPackages(core, server, appPackages)
          logger.info(`The Bot Server is in RUNNING mode...`)

          let instance = await minService.buildMin(server, appPackages)
          logger.info(`Instance loaded: ${instance.botId}...`)
          return core
        } catch (err) {
          logger.info(err)
        }

      })()
    })
  }
}

// First line to run.

GBServer.run()