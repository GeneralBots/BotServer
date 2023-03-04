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

import { GBDialogStep, GBLog, GBMinInstance, IGBCoreService, IGBPackage } from 'botlib';
import { GuaribasSchedule } from '../core.gbapp/models/GBModel.js';
import { Sequelize } from 'sequelize-typescript';
import { DialogKeywords } from './services/DialogKeywords.js';
import { SystemKeywords } from './services/SystemKeywords.js';
import { WebAutomationServices } from './services/WebAutomationServices.js';
import { ImageProcessingServices } from './services/ImageProcessingServices.js';
import { DebuggerService } from './services/DebuggerService.js';
import Koa from 'koa';
import {createRpcServer, createRpcClient} from "@push-rpc/core"
import {createKoaHttpMiddleware, createExpressHttpMiddleware, createHttpClient} from "@push-rpc/http"
import { GBServer } from '../../src/app.js';
const app = new Koa();
import {SocketServer} from "@push-rpc/core"
import * as koaBody from "koa-body"


export function createKoaHttpServer(
  port: number,
  getRemoteId: (ctx: Koa.Context) => string
): SocketServer {
  const {onError, onConnection, middleware} = createKoaHttpMiddleware(getRemoteId)

  const app = new Koa()
  app.use(koaBody.koaBody({multipart: true}))
  app.use(middleware)
  const server = app.listen(port)

  return {
    onError,
    onConnection,
    close(cb) {
      server.close(cb)
    },
  }
}





/**
 * Package for core.gbapp.
 */

export class GBBasicPackage implements IGBPackage {
  public sysPackages: IGBPackage[];
  public CurrentEngineName = 'guaribas-1.0.0';

  public async loadPackage (core: IGBCoreService, sequelize: Sequelize): Promise<void> {
    core.sequelize.addModels([GuaribasSchedule]);

    const dk = new DialogKeywords();
    const wa = new WebAutomationServices();
    const sys = new SystemKeywords();
    const dbg = new DebuggerService();
    const img = new ImageProcessingServices();

// remote id is required for assigning separate HTTP requests to a single session 
function getRemoteId(ctx: Koa.Context) {
  return "1" // share a single session for now, real impl could use cookies or some other meaning for HTTP sessions
}

      



     GBServer.globals.server.dk = createRpcServer(dk, createKoaHttpServer(5555, getRemoteId),{
      listeners: {
        unsubscribed(subscriptions: number): void {},
        subscribed(subscriptions: number): void {},
        disconnected(remoteId: string, connections: number): void {
          console.log(`Client ${remoteId} disconnected`)
        },
        connected(remoteId: string, connections: number): void {
          console.log(`New client ${remoteId} connected`)
        },
        messageIn(...params): void {
          console.log("IN ", params)
        },
        messageOut(...params): void {
          console.log("OUT ", params)
        },
      },
      pingSendTimeout: null,
      keepAliveTimeout: null,
    })
    
    console.log("RPC Server started at ws://localhost:5555")

    // GBServer.globals.wa = createRpcServer(wa, createWebsocketServer({port: 1112}));
    // GBServer.globals.sys = createRpcServer(sys, createWebsocketServer({port: 1113}));
    // GBServer.globals.dbg = createRpcServer(dbg, createWebsocketServer({port: 1114}));
    // GBServer.globals.img = createRpcServer(img, createWebsocketServer({port: 1115}));
  }

  public async getDialogs (min: GBMinInstance) {
    GBLog.verbose(`getDialogs called.`);
  }
  public async unloadPackage (core: IGBCoreService): Promise<void> {
    GBLog.verbose(`unloadPackage called.`);
  }
  public async unloadBot (min: GBMinInstance): Promise<void> {
    GBLog.verbose(`unloadBot called.`);
  }
  public async onNewSession (min: GBMinInstance, step: GBDialogStep): Promise<void> {
    GBLog.verbose(`onNewSession called.`);
  }
  public async onExchangeData (min: GBMinInstance, kind: string, data: any) {
    GBLog.verbose(`onExchangeData called.`);
  }
  public async loadBot (min: GBMinInstance): Promise<void> {

    const botId = min.botId;
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
