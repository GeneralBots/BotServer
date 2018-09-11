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

const logger = require("../../../src/logger")

"use strict"

export class GBConfigService {
  static init(): any {
    try {
      require("dotenv-extended").load({
        path: ".env",
        errorOnMissing: true,
        errorOnExtra: false,
        overrideProcessEnv: true
      })
    } catch (e) {
      console.error(e.message)
      process.exit(3)
    }
  }

  static get(key: string): string | undefined {
    let value = process.env["container:" + key]

    if (!value) {
      value = process.env[key]
    }

    if (!value) {
      switch (key) {
        case "DATABASE_DIALECT":
          value = "sqlite"
          break

        case "DATABASE_STORAGE":
          value = "./guaribas.sqlite"
          break

        case "ADDITIONAL_DEPLOY_PATH":
          value = undefined
          break

        case "DATABASE_SYNC":
        case "DATABASE_SYNC_ALTER":
        case "DATABASE_SYNC_FORCE":
          value = "false"
          break

        case "DATABASE_LOGGING":
          value = "false"
          break

        case "DATABASE_ENCRYPT":
          value = "true"
          break

        default:
          logger.info(
            `Guaribas General Error: Invalid key on .env file: '${key}'`
          )
          break
      }
    }
    return value
  }
}
