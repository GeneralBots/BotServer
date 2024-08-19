/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.          |
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
| "General Bots" is a registered trademark of pragmatismo.cloud.              |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

'use strict';

import { GBLog, GBMinInstance } from 'botlib';
import { GBServer } from '../../../src/app.js';
import Fs from 'fs';
import SwaggerClient from 'swagger-client';
import { spawn } from 'child_process';
import { CodeServices } from '../../gpt.gblib/services/CodeServices.js';
import { GBLogEx } from '../../core.gbapp/services/GBLogEx.js';
import { GBUtil } from '../../../src/util.js';

/**
 * Web Automation services of conversation to be called by BASIC.
 */
export class DebuggerService {

  public async setBreakpoint({ botId, line }) {
    GBLogEx.info(botId, `BASIC: Enabled breakpoint for ${botId} on ${line}.`);
    GBServer.globals.debuggers[botId].breaks.push(Number.parseInt(line));
  }

  public async refactor({ botId, code, change }) {
    const service = new CodeServices();
    return await service.refactor(code, change);
  }

  public async resume({ botId }) {
    if (GBServer.globals.debuggers[botId].state === 2) {
      const client = GBServer.globals.debuggers[botId].client;
      await client.Debugger.resume();
      GBServer.globals.debuggers[botId].state = 1;
      GBServer.globals.debuggers[botId].stateInfo = 'Running (Debug)';
      return { status: 'OK' };
    } else {
      const error = 'Invalid call to resume and state not being debug(2).';
      return { error: error };
    }
  }

  public async stop({ botId }) {
    GBServer.globals.debuggers[botId].state = 0;
    GBServer.globals.debuggers[botId].stateInfo = 'Stopped';

    const kill = ref => {
      spawn('sh', ['-c', `pkill -9 -f ${ref}`]);
    };

    kill(GBServer.globals.debuggers[botId].childProcess);

    return { status: 'OK' };
  }

  public async step({ botId }) {
    if (GBServer.globals.debuggers[botId].state === 2) {
      GBServer.globals.debuggers[botId].stateInfo = 'Break';
      const client = GBServer.globals.debuggers[botId].client;
      await client.Debugger.stepOver();
      return { status: 'OK' };
    } else {
      const error = 'Invalid call to stepOver and state not being debug(2).';
      return { error: error };
    }
  }

  public async getContext({ botId }) {
    const conversationsMap = GBServer.globals.debuggers[botId].conversationsMap;
    const watermarkMap = GBServer.globals.debuggers[botId].watermarkMap;

    const conversationId = conversationsMap[botId];
    let messages = [];
    const client = GBServer.globals.debuggers[botId].client;
    if (client) {
      const response = await client.apis.Conversations.Conversations_GetActivities({
        conversationId: conversationId,
        watermark: watermarkMap[botId]
      });
      watermarkMap[botId] = response.obj.watermark;
      let activities = response.obj.activites;

      if (activities && activities.length) {
        activities = activities.filter(m => m.from.id === botId && m.type === 'message');
        if (activities.length) {
          activities.forEach(activity => {
            messages.push({ text: activity.text });
            GBLogEx.info(botId, `Debugger sending text to API: ${activity.text}`);
          });
        }
      }
    }

    let messagesText = messages.join('\n');

    return {
      status: 'OK',
      state: GBServer.globals.debuggers[botId].state,
      messages: messagesText,
      scope: GBServer.globals.debuggers[botId].scope,
      scopeInfo: GBServer.globals.debuggers[botId].stateInfo
    };
  }

  public async start({ botId, botApiKey, scriptName }) {
    const conversationsMap = GBServer.globals.debuggers[botId].conversationsMap;

    let error;
    if (!GBServer.globals.debuggers[botId]) {
      GBServer.globals.debuggers[botId] = {};
    }

    if (!scriptName) {
      scriptName = 'start';
    }

    if (GBServer.globals.debuggers[botId].state === 1) {
      error = `Cannot DEBUG an already running process. ${botId}`;
      return { error: error };
    } else if (GBServer.globals.debuggers[botId].state === 2) {
      GBLogEx.info(botId, `BASIC: Releasing execution ${botId} in DEBUG mode.`);
      await this.resume({ botId });
      return { status: 'OK' };
    } else {
      GBLogEx.info(botId, `BASIC: Running ${botId} in DEBUG mode.`);
      GBServer.globals.debuggers[botId].state = 1;
      GBServer.globals.debuggers[botId].stateInfo = 'Running (Debug)';

      let min: GBMinInstance = GBServer.globals.minInstances.filter(p => p.instance.botId === botId)[0];

      const client = await GBUtil.getDirectLineClient(min);

      GBServer.globals.debuggers[botId].client = client;
      const response = await client.apis.Conversations.Conversations_StartConversation();
      const conversationId = response.obj.conversationId;
      GBServer.globals.debuggers[botId].conversationId = conversationId;

      client.apis.Conversations.Conversations_PostActivity({
        conversationId: conversationId,
        activity: {
          textFormat: 'plain',
          text: `/calldbg ${scriptName}`,
          type: 'message',
          from: {
            id: 'word',
            name: 'word'
          }
        }
      });

      return { status: 'OK' };
    }
  }

  public async sendMessage({ botId, botApiKey, text }) {
    const conversationsMap = GBServer.globals.debuggers[botId].conversationsMap;

    let error;
    if (!GBServer.globals.debuggers[botId]) {
      GBServer.globals.debuggers[botId] = {};
    }

    if (GBServer.globals.debuggers[botId].state != 1) {
      error = `Cannot sendMessage to an stopped process. ${botId}`;
      return { error: error };
    }

    let min: GBMinInstance = GBServer.globals.minInstances.filter(p => p.instance.botId === botId)[0];

      const client  = GBServer.globals.debuggers[botId].client;
      const conversationId = GBServer.globals.debuggers[botId].conversationId;

      client.apis.Conversations.Conversations_PostActivity({
        conversationId: conversationId,
        activity: {
          textFormat: 'plain',
          text: text,
          type: 'message',
          from: {
            id: 'word',
            name: 'word'
          }
        }
      });

      return { status: 'OK' };
  }

}
