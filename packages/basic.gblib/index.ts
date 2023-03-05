/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' v `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| (˅) |( (_) )  |
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

import { GBDialogStep, GBLog, GBMinInstance, IGBCoreService, IGBInstance, IGBPackage } from 'botlib';
import { GuaribasSchedule } from '../core.gbapp/models/GBModel.js';
import { Sequelize } from 'sequelize-typescript';
import { DialogKeywords } from './services/DialogKeywords.js';
import { SystemKeywords } from './services/SystemKeywords.js';
import { WebAutomationServices } from './services/WebAutomationServices.js';
import { ImageProcessingServices } from './services/ImageProcessingServices.js';
import { DebuggerService } from './services/DebuggerService.js';
import Koa from 'koa';
import { createRpcServer, createRpcClient } from '@push-rpc/core';
import { createHttpKoaMiddleware, createHttpClient } from '@push-rpc/http';
import { HttpServerOptions } from '@push-rpc/http/dist/server.js';
import { GBServer } from '../../src/app.js';
const app = new Koa();
import { SocketServer } from '@push-rpc/core';
import * as koaBody from 'koa-body';
import { GBVMService } from './services/GBVMService.js';
import { GBLogEx } from '../core.gbapp/services/GBLogEx.js';
import { CollectionUtil } from 'pragmatismo-io-framework';

export function createKoaHttpServer(
  port: number,
  getRemoteId: (ctx: Koa.Context) => string,
  opts: Partial<HttpServerOptions> = {}
): SocketServer {
  const { onError, onConnection, middleware } = createHttpKoaMiddleware(getRemoteId, opts);

  const app = new Koa();
  app.use(koaBody.koaBody({ multipart: true }));
  app.use(middleware);
  const server = app.listen(port);

  return {
    onError,
    onConnection,
    close(cb) {
      server.close(cb);
    }
  };
}

/**
 * Package for core.gbapp.
 */

export class GBBasicPackage implements IGBPackage {
  public sysPackages: IGBPackage[];
  public CurrentEngineName = 'guaribas-1.0.0';

  public async loadPackage(core: IGBCoreService, sequelize: Sequelize): Promise<void> {
    core.sequelize.addModels([GuaribasSchedule]);
  }

  public async getDialogs(min: GBMinInstance) {
    GBLog.verbose(`getDialogs called.`);
  }
  public async unloadPackage(core: IGBCoreService): Promise<void> {
    GBLog.verbose(`unloadPackage called.`);
  }
  public async unloadBot(min: GBMinInstance): Promise<void> {
    GBLog.verbose(`unloadBot called.`);
  }
  public async onNewSession(min: GBMinInstance, step: GBDialogStep): Promise<void> {
    GBLog.verbose(`onNewSession called.`);
  }
  public async onExchangeData(min: GBMinInstance, kind: string, data: any) {
    GBLog.verbose(`onExchangeData called.`);
  }
  public async loadBot(min: GBMinInstance): Promise<void> {
    const botId = min.botId;

    const opts = {
      pingSendTimeout: null,
      keepAliveTimeout: null,
      listeners: {
        unsubscribed(subscriptions: number): void {},
        subscribed(subscriptions: number): void {},
        disconnected(remoteId: string, connections: number): void {},
        connected(remoteId: string, connections: number): void {},
        messageIn(...params): void {
          GBLogEx.info(min, 'API IN' + params);
        },
        messageOut(...params): void {
          GBLogEx.info(min, 'API OUT ' + params);
        }
      }
    };

    function getRemoteId(ctx: Koa.Context) {
      return '1'; // share a single session for now, real impl could use cookies or some other meaning for HTTP sessions
    }
    let instances: IGBInstance[];
    instances = await min.core.loadInstances();
    let proxies = {};
    await CollectionUtil.asyncForEach(instances, async instance => {
      const proxy = {
        dk: new DialogKeywords(),
        wa: new WebAutomationServices(),
        sys: new SystemKeywords(),
        dbg: new DebuggerService(),
        img: new ImageProcessingServices()
      };
      proxies[instance.botId] = proxy;
    });

    GBServer.globals.server.dk = createRpcServer(
      proxies,
      createKoaHttpServer(GBVMService.API_PORT, getRemoteId, { prefix: `api/v3` }),
      opts
    );

    GBLogEx.info(min, 'API RPC HTTP Server started at http://localhost:' + GBVMService.API_PORT);

    GBServer.globals.debuggers[botId] = {};
    GBServer.globals.debuggers[botId].state = 0;
    GBServer.globals.debuggers[botId].breaks = [];
    GBServer.globals.debuggers[botId].stateInfo = 'Stopped';
    GBServer.globals.debuggers[botId].childProcess = null;
    GBServer.globals.debuggers[botId].client = null;
    GBServer.globals.debuggers[botId].conversationsMap = {};
    GBServer.globals.debuggers[botId].watermarkMap = {};
  }
}
