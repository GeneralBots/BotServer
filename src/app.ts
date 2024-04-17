/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.com.br. All rights reserved.         |
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
| "General Bots" is a registered trademark of pragmatismo.com.br.             |
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
import http from 'http';
import mkdirp from 'mkdirp';
import Path from 'path';
import swaggerUI from 'swagger-ui-dist';
import path from 'path';
import fs from 'fs';
import { GBLog, GBMinInstance, IGBCoreService, IGBInstance, IGBPackage } from 'botlib';
import { GBAdminService } from '../packages/admin.gbapp/services/GBAdminService.js';
import { AzureDeployerService } from '../packages/azuredeployer.gbapp/services/AzureDeployerService.js';
import { GBConfigService } from '../packages/core.gbapp/services/GBConfigService.js';
import { GBConversationalService } from '../packages/core.gbapp/services/GBConversationalService.js';
import { GBCoreService } from '../packages/core.gbapp/services/GBCoreService.js';
import { GBDeployer } from '../packages/core.gbapp/services/GBDeployer.js';
import { GBImporter } from '../packages/core.gbapp/services/GBImporterService.js';
import { GBMinService } from '../packages/core.gbapp/services/GBMinService.js';
import auth from 'basic-auth';
import { ChatGPTAPIBrowser } from 'chatgpt';
import child_process from 'child_process';
import * as winston from 'winston-logs-display';
import { RootData } from './RootData.js';
import { GBSSR } from '../packages/core.gbapp/services/GBSSR.js';
import { Mutex } from 'async-mutex';
import httpProxy from 'http-proxy';

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

    if (process.env.TEST_SHELL) {
      GBLog.info(`Running TEST_SHELL: ${process.env.TEST_SHELL}...`);
      try {
        child_process.execSync(process.env.TEST_SHELL);
      } catch (error) {
        GBLog.error(`Running TEST_SHELL ERROR: ${error}...`);
      }
    }

    const server = express();
    this.initEndpointsDocs(server);

    GBServer.globals.server = server;
    GBServer.globals.httpsServer = null;
    GBServer.globals.webSessions = {};
    GBServer.globals.processes = {};
    GBServer.globals.files = {};
    GBServer.globals.appPackages = [];
    GBServer.globals.sysPackages = [];
    GBServer.globals.minInstances = [];
    GBServer.globals.minBoot = new GBMinInstance();
    GBServer.globals.wwwroot = null;
    GBServer.globals.entryPointDialog = null;
    GBServer.globals.debuggers = [];
    GBServer.globals.indexSemaphore = new Mutex();

    server.use(bodyParser.json());
    server.use(bodyParser.json({ limit: '1mb' }));
    server.use(bodyParser.urlencoded({ limit: '1mb', extended: true }));

    process.on('SIGTERM', () => {
      GBLog.info('SIGTERM signal received.');
    });

    process.on('uncaughtException', (err, p) => {
      if (err !== null) {
        err = err['e'] ? err['e'] : err;
        const msg = `${err['code'] ? err['code'] : ''} ${err?.['response']?.['data'] ? err?.['response']?.['data']: ''} ${err.message ? err.message : ''} ${err['description'] ? err['description'] : ''}`
        GBLog.error(`UNCAUGHT_EXCEPTION:  ${err.toString()} ${err['stack'] ? '\n' + err['stack'] : ''} ${msg}`);
      } else {
        GBLog.error('UNCAUGHT_EXCEPTION: Unknown error (err is null)');
      }
    });
    // Creates working directory.

    process.env.PWD = process.cwd();
    const workDir = Path.join(process.env.PWD, 'work');
    if (!fs.existsSync(workDir)) {
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
          let azureDeployer: AzureDeployerService;

          // Ensure that local proxy is setup.

          if (process.env.NODE_ENV === 'development') {
            const proxy = GBConfigService.get('BOT_URL');
            if (proxy !== undefined) {
              GBServer.globals.publicAddress = proxy;
            } else {
              GBServer.globals.publicAddress = await core.ensureProxy(port);
              process.env.BOT_URL = GBServer.globals.publicAddress;
              GBLog.info(`Auto-proxy address at: ${process.env.BOT_URL}...`);
            }
          } else {
            const serverAddress = process.env.BOT_URL;
            GBLog.info(`.env address at ${serverAddress}...`);
            GBServer.globals.publicAddress = serverAddress;
          }


          // Creates a boot instance or load it from storage.

          let runOnce = false;
          if (GBConfigService.get('STORAGE_SERVER')) {
            azureDeployer = await AzureDeployerService.createInstance(deployer);
            await core.initStorage();
          } else {
            runOnce = true;
            [GBServer.globals.bootInstance, azureDeployer] = await core['createBootInstanceEx'](
              core,
              null,
              GBServer.globals.publicAddress,
              deployer,
              GBConfigService.get('FREE_TIER')
            );
          }

          core.ensureAdminIsSecured();

          // Deploys system and user packages.

          GBLog.info(`Deploying System packages...`);
          GBServer.globals.sysPackages = await core.loadSysPackages(core);
          GBLog.info(`Connecting to Bot Storage...`);
          await core.checkStorage(azureDeployer);
          await deployer.deployPackages(core, server, GBServer.globals.appPackages);
          await core.syncDatabaseStructure();

          if (runOnce) {
            await core.saveInstance(GBServer.globals.bootInstance);
          }

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

            instances.push(instance);
            GBServer.globals.minBoot.instance = instances[0];
            GBServer.globals.bootInstance = instances[0];
            await deployer.deployBotFull(instance, GBServer.globals.publicAddress);

            // Runs the search even with empty content to create structure.

            await azureDeployer['runSearch'](instance);
          }

          GBServer.globals.bootInstance = instances[0];

          // Builds minimal service infrastructure.

          const conversationalService: GBConversationalService = new GBConversationalService(core);
          const adminService: GBAdminService = new GBAdminService(core);
          const minService: GBMinService = new GBMinService(core, conversationalService, adminService, deployer);
          GBServer.globals.minService = minService;
          await minService.buildMin(instances);

          if (process.env.OPENAI_EMAIL) {
            if (!GBServer.globals.chatGPT) {
              GBServer.globals.chatGPT = new ChatGPTAPIBrowser({
                email: process.env.OPENAI_EMAIL,
                password: process.env.OPENAI_PASSWORD,
                markdown: false
              });
              await GBServer.globals.chatGPT.init();
            }
          }

          if (process.env.ENABLE_WEBLOG) {
            // If global log enabled, reorders transports adding web logging.

            const loggers = GBLog.getLogger();
            winston.default(server, loggers[1]);
          }


          server.post('*', async (req, res, next) => {

            const host = req.headers.host;

            // Roteamento com base no domínio.

            if (host === process.env.API_HOST) {
              GBLog.info(`Redirecting to API...`);
              return httpProxy.web(req, res, { target: 'http://localhost:1111' }); // Express server
            }

            await GBSSR.ssrFilter(req, res, next);

          });

          server.get('*', async (req, res, next) => {

            const host = req.headers.host;

            // Roteamento com base no domínio.

            if (host === process.env.API_HOST) {
              GBLog.info(`Redirecting to API...`);
              return httpProxy.web(req, res, { target: 'http://localhost:1111' }); // Express server
            }

            if (req.originalUrl.startsWith('/logs')) {
              if (process.env.ENABLE_WEBLOG === "true") {
                const admins = {
                  admin: { password: process.env.ADMIN_PASS }
                };

                // ... some not authenticated middlewares.
                const user = auth(req);
                if (!user || !admins[user.name] || admins[user.name].password !== user.pass) {
                  res.set('WWW-Authenticate', 'Basic realm="example"');
                  return res.status(401).send();
                }
              } else {
                await GBSSR.ssrFilter(req, res, next);
              }
            } else {
              await GBSSR.ssrFilter(req, res, next);
            }
          });

          GBLog.info(`The Bot Server is in RUNNING mode...`);

          // Opens Navigator.

          if (process.env.DEV_OPEN_BROWSER) {
            core.openBrowserInDevelopment();
          }
        } catch (err) {
          GBLog.error(`STOP: ${err.message ? err.message : err} ${err.stack ? err.stack : ''}`);
          process.exit(1);
        }
      })();
    };

    
    if (process.env.CERTIFICATE_PFX) {
      
      // Setups unsecure http redirect.
      
      const server1 = http.createServer((req, res) => {
        const host = req.headers.host.startsWith('www.') ?
          req.headers.host.substring(4) : req.headers.host;
        res.writeHead(301, {
          Location: "https://" + host + req.url
        }).end();
      });
      server1.listen(80);

      const options1 = {
        passphrase: process.env.CERTIFICATE_PASSPHRASE,
        pfx: fs.readFileSync(process.env.CERTIFICATE_PFX)
      };

      const httpsServer = https.createServer(options1, server).listen(port, mainCallback);
      GBServer.globals.httpsServer = httpsServer;

      for (let i = 2; ; i++) {
        const certPfxEnv = `CERTIFICATE${i}_PFX`;
        const certPassphraseEnv = `CERTIFICATE${i}_PASSPHRASE`;
        const certDomainEnv = `CERTIFICATE${i}_DOMAIN`;

        if (process.env[certPfxEnv] && process.env[certPassphraseEnv] && process.env[certDomainEnv]) {
          const options = {
            passphrase: process.env[certPassphraseEnv],
            pfx: fs.readFileSync(process.env[certPfxEnv])
          };
          httpsServer.addContext(process.env[certDomainEnv], options);
        } else {
          break;
        }
      }
    }
    else {
      server.listen(port, mainCallback);
    }
  }

  public static initEndpointsDocs(app: express.Application) {
    const ENDPOINT = '/docs';
    const SWAGGER_FILE_NAME = 'swagger.yaml';
    const swaggerUiAssetPath = swaggerUI.getAbsoluteFSPath();

    // A workaround for swagger-ui-dist not being able to set custom swagger URL
    const indexContent = fs
      .readFileSync(path.join(swaggerUiAssetPath, 'swagger-initializer.js'))
      .toString()
      .replace('https://petstore.swagger.io/v2/swagger.json', `/${SWAGGER_FILE_NAME}`);
    app.get(`${ENDPOINT}/swagger-initializer.js`, (req, res) => res.send(indexContent));

    // Serve the swagger-ui assets
    app.use(ENDPOINT, express.static(swaggerUiAssetPath));

    // Serve the swagger file
    app.get(`/${SWAGGER_FILE_NAME}`, (req, res) => {
      res.sendFile(path.join(process.env.PWD, SWAGGER_FILE_NAME));
    });
  }
}
