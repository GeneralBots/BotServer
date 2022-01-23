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

const express = require('express');
const bodyParser = require('body-parser');
const https = require('https');
const mkdirp = require('mkdirp');
const Path = require('path');

import * as fs from 'fs';
import { GBLog, GBMinInstance, IGBCoreService, IGBInstance, IGBPackage } from 'botlib';
import { GBAdminService } from '../packages/admin.gbapp/services/GBAdminService';
import { AzureDeployerService } from '../packages/azuredeployer.gbapp/services/AzureDeployerService';
import { GBConfigService } from '../packages/core.gbapp/services/GBConfigService';
import { GBConversationalService } from '../packages/core.gbapp/services/GBConversationalService';
import { GBCoreService } from '../packages/core.gbapp/services/GBCoreService';
import { GBDeployer } from '../packages/core.gbapp/services/GBDeployer';
import { GBImporter } from '../packages/core.gbapp/services/GBImporterService';
import { GBMinService } from '../packages/core.gbapp/services/GBMinService';


/**
 * Global shared server data;
 */
export class RootData {
  public publicAddress: string; // URI for BotServer
  public server: any; // Express reference
  public sysPackages: any[]; // Loaded system package list
  public appPackages: any[]; // Loaded .gbapp package list
  public minService: GBMinService; // Minimalist service core
  public bootInstance: IGBInstance; // General Bot Interface Instance
  public minInstances: any[]; // List of bot instances.
  public minBoot: GBMinInstance; // Reference to boot bot.
  public wwwroot: string; // .gbui or a static webapp.
  public entryPointDialog: string; // To replace default welcome dialog.
}
/**
 * General Bots open-core entry point.
 */
export class GBServer {
  public static globals: RootData;

  /**
   *  Program entry-point.
   */

  public static run() {


    GBLog.info(`The Bot Server is in STARTING mode...`);
    GBServer.globals = new RootData();
    GBConfigService.init();
    const port = GBConfigService.getServerPort();
    const server = express();
    GBServer.globals.server = server;
    GBServer.globals.appPackages = [];
    GBServer.globals.sysPackages = [];
    GBServer.globals.minInstances = [];
    GBServer.globals.wwwroot = null;
    GBServer.globals.entryPointDialog = null;

    server.use(bodyParser.json());
    server.use(bodyParser.urlencoded({ extended: true }));


    // Creates working directory.

    const workDir = Path.join(process.env.PWD, 'work');
    if (!fs.existsSync(workDir)) {
      mkdirp.sync(workDir);
    }

    const mainCallback = () => {
      (async () => {

        try {
          GBLog.info(`Now accepting connections on ${port}...`);

          // Reads basic configuration, initialize minimal services.

          const core: IGBCoreService = new GBCoreService();
          const importer: GBImporter = new GBImporter(core);
          const deployer: GBDeployer = new GBDeployer(core, importer);
          const azureDeployer: AzureDeployerService = new AzureDeployerService(deployer);
          const adminService: GBAdminService = new GBAdminService(core);



          if (process.env.NODE_ENV === 'development') {
            const proxy = GBConfigService.get('REVERSE_PROXY');
            if (proxy !== undefined) {
              GBServer.globals.publicAddress = proxy;
            } else {
              // Ensure that local development proxy is setup.

              GBLog.info(`Establishing a development local proxy (ngrok)...`);
              GBServer.globals.publicAddress = await core.ensureProxy(port);
            }
            process.env.BOT_URL = GBServer.globals.publicAddress;
          } else {
            const serverAddress = process.env.BOT_URL;
            GBLog.info(`Defining server address at ${serverAddress}...`);
            GBServer.globals.publicAddress = serverAddress;
          }

          // Creates a boot instance or load it from storage.

          try {
            await core.initStorage();
          } catch (error) {
            GBLog.verbose(`Error initializing storage: ${error}`);
            GBServer.globals.bootInstance =
              await core.createBootInstance(core, azureDeployer, GBServer.globals.publicAddress);

          }

          core.ensureAdminIsSecured();

          // Deploys system and user packages.

          GBLog.info(`Deploying packages...`);
          GBServer.globals.sysPackages = await core.loadSysPackages(core);
          GBLog.info(`Connecting to Bot Storage...`);
          await core.checkStorage(azureDeployer);
          await deployer.deployPackages(core, server, GBServer.globals.appPackages);
          await core.syncDatabaseStructure();

          // Deployment of local applications for the first time.

          if (GBConfigService.get('DISABLE_WEB') !== 'true') {
            deployer.setupDefaultGBUI();
          }

          GBLog.info(`Publishing instances...`);
          const instances: IGBInstance[] = await core.loadAllInstances(
            core,
            azureDeployer,
            GBServer.globals.publicAddress
          );

          if (instances.length === 0) {

            const instance = await importer.importIfNotExistsBotPackage(
              GBConfigService.get('BOT_ID'),
              'boot.gbot',
              'packages/boot.gbot',
              GBServer.globals.bootInstance
            );
            await deployer.deployBotFull(instance, GBServer.globals.publicAddress);
            instances.push(instance);

            // Runs the search even with empty content to create structure.

            await azureDeployer['runSearch'](instance);
          }

          GBServer.globals.bootInstance = instances[0];

          // Builds minimal service infrastructure.

          const conversationalService: GBConversationalService = new GBConversationalService(core);
          const minService: GBMinService = new GBMinService(core, conversationalService, adminService, deployer);
          GBServer.globals.minService = minService;
          await minService.buildMin(instances);


          GBLog.info(`The Bot Server is in RUNNING mode...`);

          // Opens Navigator.

          // TODO: Config: core.openBrowserInDevelopment();
        } catch (err) {
          GBLog.error(`STOP: ${err.message ? err.message : err} ${err.stack ? err.stack : ''}`);
          process.exit(1);
        }
      })();
    };
    if (process.env.ENABLE_HTTPS) {
      const sslOptions = {
        key: fs.readFileSync('certificates/gb.key', 'utf8'),
        cert: fs.readFileSync('certificates/gb.crt', 'utf8'),
        ca: fs.readFileSync('certificates/gb-ca.crt', 'utf8'),
      };

      https.createServer(sslOptions, server).listen(port);
    }
    else {
      server.listen(port, mainCallback);
    }
  }
}
