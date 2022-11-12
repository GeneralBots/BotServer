/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' v `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__,\| (˅) |( (_) )  |
|   | ,__/'(_)  `\__,_)`\__  |(_) (_) (_)`\__,_)`\__)(_)(____/(_) (_)`\___/'  |
|   | |                ( )_) |                                                |
|   (_)                 \___/'                                                |
|                                                                             |
| General Bots Copyright (c) Pragmatismo.io. All rights reserved.             |
| Licensed under the AGPL-3.0.                                                |
|                                                                             |
| According to our dual licensing model,this program can be used either      |
| under the terms of the GNU Affero General Public License,version 3,       |
| or under a proprietary license.                                             |
|                                                                             |
| The texts of the GNU Affero General Public License with an additional       |
| permission and of our proprietary license can be found at and               |
| in the LICENSE file you have received along with this program.              |
|                                                                             |
| This program is distributed in the hope that it will be useful,            |
| but WITHOUT ANY WARRANTY,without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of Pragmatismo.io.                 |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights,title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

'use strict';

import { GBLog, GBMinInstance } from 'botlib';
import { GBServer } from '../../../src/app';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService';
import { GuaribasUser } from '../../security.gbapp/models';
import { DialogKeywords } from './DialogKeywords';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer';
const Swagger = require('swagger-client');
const fs = require('fs');
import { CollectionUtil } from 'pragmatismo-io-framework';
import * as request from 'request-promise-native';

const urlJoin = require('url-join');
const Path = require('path');
const Fs = require('fs');
const url = require('url');

/**
 * Web Automation services of conversation to be called by BASIC.
 */
export class DebuggerService {

  /**
  * Reference to minimal bot instance.
  */
  public min: GBMinInstance;

  /**
   * Reference to the base system keywords functions to be called.
   */
  public dk: DialogKeywords;

  /**
   * Current user object to get BASIC properties read.
   */
  public user;

  /**
   * HTML browser for conversation over page interaction.
   */
  browser: any;

  sys: any;

  /**
   * The number used in this execution for HEAR calls (useful for SET SCHEDULE).
   */
  hrOn: string;

  userId: GuaribasUser;
  debugWeb: boolean;
  lastDebugWeb: Date;

  /**
   * SYSTEM account maxLines,when used with impersonated contexts (eg. running in SET SCHEDULE).
   */
  maxLines: number = 2000;

  debugMap = {};
  conversationsMap = {};
  scopeMap = {};
  watermarkMap = {};

  /**
   * When creating this keyword facade,a bot instance is
   * specified among the deployer service.
   */
  constructor(min: GBMinInstance, user, dk) {
    this.min = min;
    this.user = user;
    this.dk = dk;

    this.debugWeb = this.min.core.getParam<boolean>(
      this.min.instance,
      'Debug Web Automation',
      false
    );
  }

  private client;

  public async setBreakpoint({ botId, botApiKey, line }) {

    const client = GBServer.globals.debuggers[botId];

    async function mainScript({ Debugger }) {
      return new Promise((fulfill, reject) => {
        Debugger.scriptParsed((params) => {
          const { scriptId, url } = params;
          fulfill(scriptId);
        });
      });
    }

    const scriptId = await mainScript(client);
    const { breakpointId } = await await client.Debugger.setBreakpoint({
      location: {
        scriptId,
        lineNumber: line - 1
      }
    });
  }

  public async removeBreakPoint({ botId, botApiKey, line }) {

  }

  public async continueRun({ botId, botApiKey, force }) {
    const client = GBServer.globals.debuggers[botId];
    client.Debugger.resume();
  }

  public async stop({ botId, botApiKey, force }) {
    const client = GBServer.globals.debuggers[botId];
    client.close();
  }

  public async stepOver({ botId, botApiKey }) {
    const client = GBServer.globals.debuggers[botId];
    client.stepOver();
  }

  public async getExecutionContext({ botId, botApiKey, force }) {

    const client = GBServer.globals.debuggers[botId];
    const conversationId = this.conversationsMap[botId];


    const response = await client.Conversations.Conversations_GetActivities({
      conversationId: conversationId,
      watermark: this.watermarkMap[botId]
    });
    this.watermarkMap[botId] = response.obj.watermark;
    let activities = response.obj.activites;
    let messages = [];
    if (activities && activities.length) {
      activities = activities.filter(m => m.from.id === botId && m.type === 'message');
      if (activities.length) {
        activities.forEach(activity => {
          messages.push({ text: activity.text });
          GBLog.info(`GBDEBUG: SND TO WORD ${activity.text}`);
        });
      }
    }
    return { state:this.debugMap[botId].state, messages, scope: this.scopeMap[botId] };
  }

  public async run({ botId, botApiKey, scriptName }) {

    this.debugMap[botId] = { state: 1 };

    let min: GBMinInstance = GBServer.globals.minInstances.filter(
      p => p.instance.botId === botId
    )[0];

    this.client = await new Swagger({
      spec: JSON.parse(fs.readFileSync('directline-3.0.json', 'utf8')), usePromise: true
    });
    this.client.clientAuthorizations.add(
      'AuthorizationBotConnector',
      new Swagger.ApiKeyAuthorization('Authorization', `Bearer ${min.instance.webchatKey}`, 'header')
    );
    const response = await this.client.Conversations.Conversations_StartConversation();
    const conversationId = response.obj.conversationId;
    this.conversationsMap[botId] = conversationId;
    GBServer.globals.debugConversationId = conversationId;

    this.client.Conversations.Conversations_PostActivity({
      conversationId: conversationId,
      activity: {
        textFormat: 'plain',
        text: `/call ${scriptName}`,
        type: 'message',
        from: {
          id: 'test',
          name: 'test'
        }
      }
    });

    // Setup debugger.

    const client = GBServer.globals.debuggers[botId];

    client.Debugger.paused(({ callFrames, reason, hitBreakpoints }) => {

      const frame = callFrames[0];
      if (hitBreakpoints.length > 1) {
        GBLog.info(`.gbdialog break at line ${frame.location.lineNumber + 1}`); // (zero-based)

        const scope = `${frame.scopeChain[0].name} ${frame.scopeChain[0].object}`;

        this.scopeMap[botId] = scope;
      }
      else if (reason === ''){
        GBLog.info(`.gbdialog ${reason} at line ${frame.location.lineNumber + 1}`); // (zero-based)
      }
    });

    await client.Runtime.runIfWaitingForDebugger();
    await client.Debugger.enable();
    await client.Debugger.setPauseOnExceptions('all');
  }
}