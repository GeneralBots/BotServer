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

/**
 * @fileoverview General Bots server core.
 */

'use strict';

import { Mutex } from 'async-mutex';
import auth from 'basic-auth';
import bodyParser from 'body-parser';
import { GBLog, GBMinInstance, IGBCoreService, IGBInstance } from 'botlib';
import child_process from 'child_process';
import express from 'express';
import fs from 'fs';
import http from 'http';
import httpProxy from 'http-proxy';
import https from 'https';
import mkdirp from 'mkdirp';
import { default as Path, default as path } from 'path';
import swaggerUI from 'swagger-ui-dist';
import { GBAdminService } from '../packages/admin.gbapp/services/GBAdminService.js';
import { AzureDeployerService } from '../packages/azuredeployer.gbapp/services/AzureDeployerService.js';
import { GBConfigService } from '../packages/core.gbapp/services/GBConfigService.js';
import { GBConversationalService } from '../packages/core.gbapp/services/GBConversationalService.js';
import { GBCoreService } from '../packages/core.gbapp/services/GBCoreService.js';
import { GBDeployer } from '../packages/core.gbapp/services/GBDeployer.js';
import { GBImporter } from '../packages/core.gbapp/services/GBImporterService.js';
import { GBLogEx } from '../packages/core.gbapp/services/GBLogEx.js';
import { GBMinService } from '../packages/core.gbapp/services/GBMinService.js';
import { GBSSR } from '../packages/core.gbapp/services/GBSSR.js';
import { RootData } from './RootData.js';
import { GBUtil } from './util.js';

/**
 * General Bots open-core entry point.
 */
export class GBServer {
  public static globals: RootData;

  /**
   *  Program entry-point.
   */

  public static run() {
    GBLogEx.info(0, `The Bot Server is in STARTING mode...`);
    GBServer.globals = new RootData();
    GBConfigService.init();
    const port = GBConfigService.getServerPort();

    if (process.env.TEST_SHELL) {
      GBLogEx.info(0, `Running TEST_SHELL: ${process.env.TEST_SHELL}...`);
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
    GBServer.globals.users = [];
    GBServer.globals.indexSemaphore = new Mutex();
    
    server.use(bodyParser.json());
    server.use(bodyParser.json({ limit: '1mb' }));
    server.use(bodyParser.urlencoded({ limit: '1mb', extended: true }));
    server.use(function (req, res, next) {
      for (const key in req.query) {
        req.query[key.toLowerCase()] = req.query[key];
      }
      next();
    });

    process.on('SIGTERM', () => {
      GBLogEx.info(0, 'SIGTERM signal received.');
    });

    process.on('uncaughtException', (err, p) => {
      GBLogEx.error(0, `GBEXCEPTION: ${GBUtil.toYAML(err)}`);
    });

    process.on('unhandledRejection', (err, p) => {
      let bypass = false;
      let res = err['response'];
      if (res) {
        if (res?.body?.error?.message?.startsWith('Failed to send activity: bot timed out')) {
          bypass = true;
        }
      }

      if (!bypass) {
        GBLogEx.error(0, `GBREJECTION: ${GBUtil.toYAML(err)}`);
      }
    });

    // Creates working directory.

    process.env.PWD = process.cwd();
    const workDir = path.join(process.env.PWD, 'work');
    if (!fs.existsSync(workDir)) {
      mkdirp.sync(workDir);
    }

    const mainCallback = () => {
      (async () => {
        try {
          GBLogEx.info(0, `Now accepting connections on ${port}...`);

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
              GBLogEx.info(0, `Auto-proxy address at: ${process.env.BOT_URL}...`);
            }
          } else {
            const serverAddress = process.env.BOT_URL;
            GBLogEx.info(0, `.env address at ${serverAddress}...`);
            GBServer.globals.publicAddress = serverAddress;
          }

          // Creates a boot instance or load it from storage.

          if (GBConfigService.get('STORAGE_SERVER')) {
            azureDeployer = await AzureDeployerService.createInstance(deployer);
            await core.initStorage();
          } else if (!GBConfigService.get('STORAGE_NAME')) {
            await core.initStorage();
          } else {
            [GBServer.globals.bootInstance, azureDeployer] = await core['createBootInstanceEx'](
              core,
              null,
              GBServer.globals.publicAddress,
              deployer,
              GBConfigService.get('FREE_TIER')
            );
            await core.saveInstance(GBServer.globals.bootInstance);
          }

          // Deploys system and user packages.

          GBLogEx.info(0, `Deploying System packages...`);
          GBServer.globals.sysPackages = await core.loadSysPackages(core);
          GBLogEx.info(0, `Connecting to Bot Storage...`);
          await core.checkStorage(azureDeployer);
          await deployer.deployPackages(core, server, GBServer.globals.appPackages);
          await core.syncDatabaseStructure();

          // Deployment of local applications for the first time.

          if (GBConfigService.get('DISABLE_WEB') !== 'true') {
            deployer.setupDefaultGBUI();
          }

          GBLogEx.info(0, `Publishing instances...`);
          const instances: IGBInstance[] = await core.loadAllInstances(
            core,
            azureDeployer,
            GBServer.globals.publicAddress
          );

          if (instances.length === 0) {
            if (GBConfigService.get('STORAGE_NAME')) {
              const instance = await importer.importIfNotExistsBotPackage(
                GBConfigService.get('BOT_ID'),
                'boot.gbot',
                'packages/boot.gbot',
                GBServer.globals.bootInstance
              );

              instances.push(instance);
              GBServer.globals.minBoot.instance = instances[0];
              GBServer.globals.bootInstance = instances[0];
              await deployer.deployBotOnAzure(instance, GBServer.globals.publicAddress);

              // Runs the search even with empty content to create structure.

              await azureDeployer['runSearch'](instance);
            }
          }

          const conversationalService: GBConversationalService = new GBConversationalService(core);
          const adminService: GBAdminService = new GBAdminService(core);
          const minService: GBMinService = new GBMinService(core, conversationalService, adminService, deployer);
          GBServer.globals.minService = minService;

          // Just sync if not using LOAD_ONLY.

          if (!GBConfigService.get('STORAGE_NAME') && !process.env.LOAD_ONLY) {
            await core['ensureFolders'](instances, deployer);
          }
          GBServer.globals.bootInstance = instances[0];

          // Builds minimal service infrastructure.

          const minInstances = await minService.buildMin(instances);

          GBServer.globals.webDavServer = await GBCoreService.createWebDavServer(minInstances);

          server.all('*', async (req, res, next) => {
            const host = req.headers.host;

            if (req.originalUrl.startsWith('/logs')) {
              if (process.env.ENABLE_WEBLOG === 'true') {
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
              // Setups unsecure http redirect.
              const proxy = httpProxy.createProxyServer({});

              if (host === process.env.API_HOST) {
                GBLogEx.info(0, `Redirecting to API...`);
                return proxy.web(req, res, { target: 'http://localhost:1111' }); // Express server
              } else {
                await GBSSR.ssrFilter(req, res, next);
              }
            }
          });

          GBLogEx.info(0, `The Bot Server is in RUNNING mode...`);

          await minService.startSimpleTest(GBServer.globals.minBoot);

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
      const server1 = http.createServer((req, res) => {
        const host = req.headers.host.startsWith('www.') ? req.headers.host.substring(4) : req.headers.host;

        res
          .writeHead(301, {
            Location: 'https://' + host + req.url
          })
          .end();
      });
      server1.listen(80);

      const options1 = {
        passphrase: process.env.CERTIFICATE_PASSPHRASE,
        pfx: fs.readFileSync(process.env.CERTIFICATE_PFX),
        ca: fs.existsSync(process.env.CERTIFICATE_CA) ? fs.readFileSync(process.env.CERTIFICATE_CA) : null
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
    } else {
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
