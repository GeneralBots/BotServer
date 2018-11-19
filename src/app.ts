#! /usr/bin / env node
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

/**
 * @fileoverview General Bots server core.
 */

'use strict';

const logger = require('./logger');
const express = require('express');
const bodyParser = require('body-parser');
const opn = require('opn');

import { IGBInstance, IGBPackage } from 'botlib';
import { GBAdminPackage } from '../packages/admin.gbapp/index';
import { GBAdminService } from '../packages/admin.gbapp/services/GBAdminService';
import { GBAnalyticsPackage } from '../packages/analytics.gblib';
import { AzureDeployerService } from '../packages/azuredeployer.gbapp/services/AzureDeployerService';
import { GBCorePackage } from '../packages/core.gbapp';
import { GuaribasInstance } from '../packages/core.gbapp/models/GBModel';
import { GBConfigService } from '../packages/core.gbapp/services/GBConfigService';
import { GBConversationalService } from '../packages/core.gbapp/services/GBConversationalService';
import { GBCoreService } from '../packages/core.gbapp/services/GBCoreService';
import { GBDeployer } from '../packages/core.gbapp/services/GBDeployer';
import { GBImporter } from '../packages/core.gbapp/services/GBImporter';
import { GBMinService } from '../packages/core.gbapp/services/GBMinService';
import { GBCustomerSatisfactionPackage } from '../packages/customer-satisfaction.gbapp';
import { GBKBPackage } from '../packages/kb.gbapp';
import { GBSecurityPackage } from '../packages/security.gblib';
import { GBWhatsappPackage } from './../packages/whatsapp.gblib/index';

const appPackages = new Array<IGBPackage>();

/**
 * General Bots open-core entry point.
 */
export class GBServer {

  /**
   *  Program entry-point.
   */

  public static run() {

    logger.info(`The Bot Server is in STARTING mode...`);

    // Creates a basic HTTP server that will serve several URL, one for each
    // bot instance. This allows the same server to attend multiple Bot on
    // the Marketplace until GB get serverless.

    const port = process.env.port || process.env.PORT || 4242;
    const server = express();

    server.use(bodyParser.json()); // to support JSON-encoded bodies
    server.use(
      bodyParser.urlencoded({
        // to support URL-encoded bodies
        extended: true
      })
    );

    let bootInstance: IGBInstance;
    server.listen(port, () => {
      (async () => {
        try {
          logger.info(`Now accepting connections on ${port}...`);

          // Reads basic configuration, initialize minimal services.

          GBConfigService.init();
          const core = new GBCoreService();

          // Ensures cloud / on-premises infrastructure is setup.

          logger.info(`Establishing a development local proxy (ngrok)...`);
          const proxyAddress = await core.ensureProxy(port);

          const deployer = new GBDeployer(core, new GBImporter(core));
          const azureDeployer = new AzureDeployerService(deployer);

          try {
            await core.initDatabase();
          } catch (error) {
            logger.info(`Deploying cognitive infrastructure (on the cloud / on premises)...`);
            try {
              bootInstance = await azureDeployer.deployFarm(proxyAddress);
            } catch (error) {
              logger.warn(
                'In case of error, please cleanup any infrastructure objects created during this procedure and .env before running again.'
              );
              throw error;
            }
            core.writeEnv(bootInstance);
            logger.info(`File .env written, starting General Bots...`);
            GBConfigService.init();

            await core.initDatabase();
          }

          // TODO: Get .gb* templates from GitHub and download do additional deploy folder.

          // Check admin password.

          const conversationalService = new GBConversationalService(core);
          const adminService = new GBAdminService(core);
          const password = GBConfigService.get('ADMIN_PASS');

          if (!GBAdminService.StrongRegex.test(password)) {
            throw new Error(
              'Please, define a really strong password in ADMIN_PASS environment variable before running the server.'
            );
          }

          // NOTE: the semicolon is necessary before this line.
          // Loads all system packages.

          [
            GBAdminPackage,
            GBAnalyticsPackage,
            GBCorePackage,
            GBSecurityPackage,
            GBKBPackage,
            GBCustomerSatisfactionPackage,
            GBWhatsappPackage
          ].forEach(e => {
            logger.info(`Loading sys package: ${e.name}...`);
            const p = Object.create(e.prototype) as IGBPackage;
            p.loadPackage(core, core.sequelize);
          });

          // Loads all bot instances from object storage, if it's formatted.

          logger.info(`Loading instances from storage...`);
          let instances: GuaribasInstance[];
          try {
            instances = await core.loadInstances();
            const instance = instances[0];

            if (process.env.NODE_ENV === 'development') {
              logger.info(`Updating bot endpoint to local reverse proxy (ngrok)...`);

              await azureDeployer.updateBotProxy(
                instance.botId,
                instance.botId,
                `${proxyAddress}/api/messages/${instance.botId}`
              );
            }
          } catch (error) {
            if (error.parent.code === 'ELOGIN') {
              const group = GBConfigService.get('CLOUD_GROUP');
              const serverName = GBConfigService.get('STORAGE_SERVER').split(
                '.database.windows.net'
              )[0];
              await azureDeployer.openStorageFirewall(group, serverName);
            } else {
              // Check if storage is empty and needs formatting.

              const isInvalidObject =
                error.parent.number == 208 || error.parent.errno == 1; // MSSQL or SQLITE.

              if (isInvalidObject) {
                if (GBConfigService.get('STORAGE_SYNC') != 'true') {
                  throw new Error(`Operating storage is out of sync or there is a storage connection error. Try setting STORAGE_SYNC to true in .env file. Error: ${
                    error.message
                  }.`);
                } else {
                  logger.info(
                    `Storage is empty. After collecting storage structure from all .gbapps it will get synced.`
                  );
                }
              } else {
                throw new Error(`Cannot connect to operating storage: ${error.message}.`);
              }
            }
          }

          // Deploy packages and format object store according to .gbapp storage models.

          logger.info(`Deploying packages...`);
          await deployer.deployPackages(core, server, appPackages);

          // If instances is undefined here it's because storage has been formatted.
          // Load all instances from .gbot found on deploy package directory.
          if (!instances) {
            const saveInstance = new GuaribasInstance(bootInstance);
            await saveInstance.save();
            instances = await core.loadInstances();
          }

          // Setup server dynamic (per bot instance) resources and listeners.

          logger.info(`Publishing instances...`);
          const minService = new GBMinService(
            core,
            conversationalService,
            adminService,
            deployer
          );
          await minService.buildMin(server, appPackages, instances);
          logger.info(`The Bot Server is in RUNNING mode...`);

          if (process.env.NODE_ENV === 'development') {
            opn('http://localhost:4242');
          }

          return core;
        } catch (err) {
          logger.error(`STOP: ${err} ${err.stack ? err.stack : ''}`);
          process.exit(1);
        }
      })();
    });
  }
}

// First line to run.

GBServer.run();
