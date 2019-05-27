/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
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

import { GBLog, IGBCoreService, IGBInstance, IGBPackage } from 'botlib';
import { GBAdminService } from '../packages/admin.gbapp/services/GBAdminService';
import { AzureDeployerService } from '../packages/azuredeployer.gbapp/services/AzureDeployerService';
import { GBConfigService } from '../packages/core.gbapp/services/GBConfigService';
import { GBConversationalService } from '../packages/core.gbapp/services/GBConversationalService';
import { GBCoreService } from '../packages/core.gbapp/services/GBCoreService';
import { GBDeployer } from '../packages/core.gbapp/services/GBDeployer';
import { GBImporter } from '../packages/core.gbapp/services/GBImporterService';
import { GBMinService } from '../packages/core.gbapp/services/GBMinService';

const appPackages: IGBPackage[] = [];

/**
 * Global shared server data;
 */
export class RootData {
  public publicAddress: string;
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
    const port = GBConfigService.getServerPort();
    const server = express();
    server.use(bodyParser.json());
    server.use(bodyParser.urlencoded({ extended: true }));

    server.listen(port, () => {
      (async () => {
        try {
          GBLog.info(`Now accepting connections on ${port}...`);

          // Reads basic configuration, initialize minimal services.

          GBConfigService.init();
          const core: IGBCoreService = new GBCoreService();

          const importer: GBImporter = new GBImporter(core);
          const deployer: GBDeployer = new GBDeployer(core, importer);
          const azureDeployer: AzureDeployerService = new AzureDeployerService(deployer);
          const adminService: GBAdminService = new GBAdminService(core);
          const conversationalService: GBConversationalService = new GBConversationalService(core);

          if (process.env.NODE_ENV === 'development') {
            // Ensure that local development proxy is setup.

            GBLog.info(`Establishing a development local proxy (ngrok)...`);
            GBServer.globals.publicAddress = await core.ensureProxy(port);
          } else {
            const serverAddress = `https://${process.env.WEBSITE_SITE_NAME}.azurewebsites.net`;
            GBLog.info(`Defining server address at ${serverAddress}...`);
            GBServer.globals.publicAddress = serverAddress;
          }

          // Creates a boot instance or load it from storage.

          let bootInstance: IGBInstance;
          try {
            await core.initStorage();
          } catch (error) {
            GBLog.verbose(`Error initializing storage: ${error}`);
            bootInstance = await core.createBootInstance(core, azureDeployer, GBServer.globals.publicAddress);
            await core.initStorage();
          }

          core.ensureAdminIsSecured();

          // Deploys system and user packages.

          GBLog.info(`Deploying packages...`);
          core.loadSysPackages(core);
          await core.checkStorage(azureDeployer);
          await deployer.deployPackages(core, server, appPackages);

          // Loads boot bot and other instances.

          GBLog.info(`Publishing instances...`);
          const packageInstance = await importer.importIfNotExistsBotPackage(
            GBConfigService.get('CLOUD_GROUP'),
            'boot.gbot',
            'packages/boot.gbot'
          );
          if (bootInstance === undefined) {
            bootInstance = packageInstance;
          }
          // tslint:disable-next-line:prefer-object-spread
          const fullInstance = Object.assign(packageInstance, bootInstance);
          await core.saveInstance(fullInstance);
          let instances: IGBInstance[] = await core.loadAllInstances(core, azureDeployer,
            GBServer.globals.publicAddress);
          instances = await core.ensureInstances(instances, bootInstance, core);
          if (bootInstance !== undefined) {
            bootInstance = instances[0];
          }

          // Builds minimal service infrastructure.

          const minService: GBMinService = new GBMinService(core, conversationalService, adminService, deployer);
          await minService.buildMin(bootInstance, server, appPackages, instances,
            deployer, GBServer.globals.publicAddress);

          // Deployment of local applications for the first time.

          deployer.runOnce();

          GBLog.info(`The Bot Server is in RUNNING mode...`);

          // Opens Navigator.

          core.openBrowserInDevelopment();
        } catch (err) {
          GBLog.error(`STOP: ${err} ${err.stack ? err.stack : ''}`);
          process.exit(1);
        }
      })();
    });
  }
}
