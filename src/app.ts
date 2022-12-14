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

import express from 'express';
import bodyParser from 'body-parser';
import https from 'https';
import mkdirp from 'mkdirp';
import Path from 'path';
import * as Fs from 'fs';
import { GBLog, IGBCoreService, IGBInstance, IGBPackage } from 'botlib';
import { GBAdminService } from '../packages/admin.gbapp/services/GBAdminService.js';
import { AzureDeployerService } from '../packages/azuredeployer.gbapp/services/AzureDeployerService.js';
import { GBConfigService } from '../packages/core.gbapp/services/GBConfigService.js';
import { GBConversationalService } from '../packages/core.gbapp/services/GBConversationalService.js';
import { GBCoreService } from '../packages/core.gbapp/services/GBCoreService.js';
import { GBDeployer } from '../packages/core.gbapp/services/GBDeployer.js';
import { GBImporter } from '../packages/core.gbapp/services/GBImporterService.js';
import { GBMinService } from '../packages/core.gbapp/services/GBMinService.js';
import auth from 'basic-auth';
import child_process from 'child_process';
import * as winston from 'winston-logs-display';
import { RootData } from './RootData.js';

/**
 * General Bots open-core entry point.
 */
export class GBServer {
  public static globals: RootData;

  /**
   *  Program entry-point.
   */

  public static run () {
    GBLog.info(`The Bot Server is in STARTING mode...`);
    GBServer.globals = new RootData();
    GBConfigService.init();
    const port = GBConfigService.getServerPort();

    if (process.env.TEST_SHELL) {
      GBLog.info(`Running TEST_SHELL: ${process.env.TEST_SHELL}...`);
      try {
        child_process.execSync(process.env.TEST_SHELL);
      } catch (error) {
        GBLog.error(`Running TEST_SHELL ERROR: ${error}...`);
      }
    }

    const server = express();

    GBServer.globals.server = server;
    GBServer.globals.appPackages = [];
    GBServer.globals.sysPackages = [];
    GBServer.globals.minInstances = [];
    GBServer.globals.wwwroot = null;
    GBServer.globals.entryPointDialog = null;
    GBServer.globals.debuggers = [];

    server.use(bodyParser.json());
    server.use(bodyParser.urlencoded({ extended: true }));

    process.on('unhandledRejection', (err, p) => {
      console.log('An unhandledRejection occurred');
      console.log(`Rejected Promise: ${p}`);
      console.log(`Rejection: ${err}`);
    });

    // Creates working directory.

    process.env.PWD = process.cwd();
    const workDir = Path.join(process.env.PWD, 'work');
    if (!Fs.existsSync(workDir)) {
      mkdirp.sync(workDir);
    }

    const mainCallback = () => {
      (async () => {
        try {
          GBLog.info(`Now accepting connections on ${port}...`);
          process.env['NODE_TLS_REJECT_UNAUTHORIZED'] = '0';

          // Reads basic configuration, initialize minimal services.

          const core: IGBCoreService = new GBCoreService();
          const importer: GBImporter = new GBImporter(core);
          const deployer: GBDeployer = new GBDeployer(core, importer);
          const azureDeployer: AzureDeployerService = new AzureDeployerService(deployer);
          const adminService: GBAdminService = new GBAdminService(core);

          if (process.env.NODE_ENV === 'development' && !process.env.BOT_URL) {
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
            GBServer.globals.bootInstance = await core.createBootInstance(
              core,
              azureDeployer,
              GBServer.globals.publicAddress
            );
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

          if (process.env.ENABLE_WEBLOG) {
            const admins = {
              admin: { password: process.env.ADMIN_PASS }
            };

            // ... some not authenticated middlewares

            server.use(async (req, res, next) => {
              if (req.originalUrl.startsWith('/logs')) {
                const user = auth(req);
                if (!user || !admins[user.name] || admins[user.name].password !== user.pass) {
                  res.set('WWW-Authenticate', 'Basic realm="example"');
                  return res.status(401).send();
                }
              } else {
                return next();
              }
            });

            // If global log enabled, reorders transports adding web logging.

            const loggers = GBLog.getLogger();
            winston.default(server, loggers[1]);
          }

          GBLog.info(`The Bot Server is in RUNNING mode...`);

          // Opens Navigator.

          // TODO: Config: core.openBrowserInDevelopment();
        } catch (err) {
          GBLog.error(`STOP: ${err.message ? err.message : err} ${err.stack ? err.stack : ''}`);
          process.exit(1);
        }
      })();
    };
    // TODO: Move to .gbot folder myown.com pointing to generalbots.ai/myown
    if (process.env.CERTIFICATE_PFX) {
      const options1 = {
        passphrase: process.env.CERTIFICATE_PASSPHRASE,
        pfx: Fs.readFileSync(process.env.CERTIFICATE_PFX)
      };
      const httpsServer = https.createServer(options1, server).listen(port, mainCallback);

      if (process.env.CERTIFICATE2_PFX) {
        const options2 = {
          passphrase: process.env.CERTIFICATE2_PASSPHRASE,
          pfx: Fs.readFileSync(process.env.CERTIFICATE2_PFX)
        };
        httpsServer.addContext(process.env.CERTIFICATE2_DOMAIN, options2);
      }
    } else {
      server.listen(port, mainCallback);
    }
  }
}
