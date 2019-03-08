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

'use strict';

import { TurnContext } from 'botbuilder';
import { WaterfallStepContext } from 'botbuilder-dialogs';
import { GBMinInstance } from 'botlib';
import * as request from 'request-promise-native';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService';
import { AzureDeployerService } from '../../azuredeployer.gbapp/services/AzureDeployerService';
import { GBDeployer } from './GBDeployer';
const UrlJoin = require('url-join');

/**
 * BASIC system class for extra manipulation of bot behaviour.
 */
class SysClass {
  public min: GBMinInstance;
  private readonly deployer: GBDeployer;

  constructor(min: GBMinInstance, deployer: GBDeployer) {
    this.min = min;
    this.deployer = deployer;
  }

  public async wait(seconds: number) {
    // tslint:disable-next-line no-string-based-set-timeout
    const timeout = async (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
    await timeout(seconds * 1000);
  }

  public generatePassword() {
    return GBAdminService.getRndPassword();
  }

  public async createABotFarmUsing(
    botId,
    username,
    password,
    location,
    nlpAuthoringKey,
    appId,
    appPassword,
    subscriptionId
  ) {
    const service = new AzureDeployerService(this.deployer);
    await service.deployToCloud(
      botId,
      username,
      password,
      location,
      nlpAuthoringKey,
      appId,
      appPassword,
      subscriptionId
    );
  }

  /**
   * Generic function to call any REST API.
   */
  public async sendEmail(to, subject, body) {
    // tslint:disable-next-line:no-console
    console.log(`[E-mail]: to:${to}, subject: ${subject}, body: ${body}.`);
  }

  /**
   * Generic function to call any REST API.
   */
  public async httpGet(url: string, qs) {

    const options = {
        uri: UrlJoin(url , qs)
    };

    return await request.get(options);
  }

}
/**
 * @fileoverview General Bots server core.
 */

export default class DialogClass {

  public min: GBMinInstance;
  public context: TurnContext;
  public step: WaterfallStepContext;
  public internalSys: SysClass;

  constructor(min: GBMinInstance, deployer: GBDeployer) {
    this.min = min;
    this.internalSys = new SysClass(min, deployer);
  }

  public sys(): SysClass {
    return this.internalSys;
  }

  public async hear(cb) {
    const idCallback = crypto.getRandomValues(new Uint32Array(16))[0];
    this.min.cbMap[idCallback] = cb;
    await this.step.beginDialog('/hear', { id: idCallback });
  }

  public async talk(text: string) {
    return await this.context.sendActivity(text);
  }
}
