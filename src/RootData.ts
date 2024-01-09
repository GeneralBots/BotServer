/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.com.br. All rights reserved.             |
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
| "General Bots" is a registered trademark of pragmatismo.com.br.                 |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

/**
 * @fileoverview General Bots server core.
 */
'use strict';
import { GBMinInstance, IGBInstance } from 'botlib';
import { GBMinService } from '../packages/core.gbapp/services/GBMinService.js';

/**
 * Global shared server data;
 */

export class RootData {
  public webSessions: {}; // List of Web Automation sessions.
  public processes: {}; // List of .gbdialog active executions.
  public files: {}; // List of uploaded files handled.
  public publicAddress: string; // URI for BotServer.
  public server: any; // Express reference.
  public httpsServer: any; // Express reference (HTTPS).
  public apiServer: any; // Koa reference (HTTPS) for GB API (isolated from /).
  public sysPackages: any[]; // Loaded system package list.
  public appPackages: any[]; // Loaded .gbapp package list.
  public minService: GBMinService; // Minimalist service core.
  public bootInstance: IGBInstance; // General Bot Interface Instance.
  public minInstances: any[]; // List of bot instances.
  public minBoot: GBMinInstance; // Reference to boot bot.
  public wwwroot: string; // .gbui or a static webapp.
  public entryPointDialog: string; // To replace default welcome dialog.
  public debugConversationId: any; // Used to self-message during debug.
  public debuggers: any[]; // Client of attached Debugger instances by botId.
  public chatGPT: any; // ChatGPT API handle (shared Browser).
  public dk;
  public wa;
  public sys;
  public dbg;
  public img;
  indexSemaphore: any;
}
