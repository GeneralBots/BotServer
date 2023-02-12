/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___  _   _    _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/ \ /`\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| |*| |( (_) )  |
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

/**
 * @fileoverview General Bots server core.
 */

'use strict';

import { GBLog, IGBInstance } from "botlib";
import { GuaribasLog } from "../models/GBModel";

export class GBLogEx {
  public static async error(minOrInstanceId: any, message: string) {
    GBLog.error(message);
    if (typeof minOrInstanceId === 'object') {
      minOrInstanceId = minOrInstanceId.instance.instanceId;
    }
    await this.log(minOrInstanceId, 'e', message);
  }

  public static async debug(minOrInstanceId: any, message: string) {
    GBLog.debug(message);
    if (typeof minOrInstanceId === 'object') {
      minOrInstanceId = minOrInstanceId.instance.instanceId;
    }
    await this.log(minOrInstanceId, 'd', message);
  }

  public static async info(minOrInstanceId: any,  message: string) {
    GBLog.info(message);
    if (typeof minOrInstanceId === 'object') {
      minOrInstanceId = minOrInstanceId.instance.instanceId;
    }
    await this.log(minOrInstanceId, 'i', message);
  }

  public static async verbose(minOrInstanceId: any, message: string) {
    GBLog.verbose(message);
    if (typeof minOrInstanceId === 'object') {
      minOrInstanceId = minOrInstanceId.instance.instanceId;
    }
    await this.log(minOrInstanceId, 'v', message);
  }

  /**
   * Finds and update user agent information to a next available person.
   */
  public static async log(instance: IGBInstance, kind: string, message: string): Promise<GuaribasLog> {
    return await GuaribasLog.create(<GuaribasLog>{
      instanceId: instance.instanceId,
      message: message,
      kind: kind
    });
  }
}
