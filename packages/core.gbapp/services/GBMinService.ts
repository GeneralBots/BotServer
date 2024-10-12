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
import { createRpcServer } from '@push-rpc/core';
import AuthenticationContext from 'adal-node';
import arrayBufferToBuffer from 'arraybuffer-to-buffer';
import { Semaphore } from 'async-mutex';
import { Mutex } from 'async-mutex';
import chokidar from 'chokidar';
import {
  AutoSaveStateMiddleware,
  BotFrameworkAdapter,
  ConversationState,
  MemoryStorage,
  TurnContext,
  UserState
} from 'botbuilder';
import { FacebookAdapter } from 'botbuilder-adapter-facebook';
import {
  AttachmentPrompt,
  ConfirmPrompt,
  DialogSet,
  OAuthPrompt,
  TextPrompt,
  WaterfallDialog
} from 'botbuilder-dialogs';
import { MicrosoftAppCredentials } from 'botframework-connector';
import {
  GBDialogStep,
  GBLog,
  GBMinInstance,
  IGBAdminService,
  IGBConversationalService,
  IGBCoreService,
  IGBInstance,
  IGBPackage
} from 'botlib';
import cliProgress from 'cli-progress';
import removeRoute from 'express-remove-route';
import fs from 'fs/promises';
import Koa from 'koa';
import mkdirp from 'mkdirp';
import { NlpManager } from 'node-nlp';
import path from 'path';
import { CollectionUtil } from 'pragmatismo-io-framework';
import SwaggerClient from 'swagger-client';
import urlJoin from 'url-join';
import wash from 'washyourmouthoutwithsoap';
import { v2 as webdav } from 'webdav-server';
import { start as startRouter } from '../../../packages/core.gbapp/services/router/bridge.js';
import { GBServer } from '../../../src/app.js';
import { GBUtil } from '../../../src/util.js';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import { GuaribasConversationMessage } from '../../analytics.gblib/models/index.js';
import { AnalyticsService } from '../../analytics.gblib/services/AnalyticsService.js';
import { createKoaHttpServer } from '../../basic.gblib/index.js';
import { DebuggerService } from '../../basic.gblib/services/DebuggerService.js';
import { DialogKeywords } from '../../basic.gblib/services/DialogKeywords.js';
import { GBVMService } from '../../basic.gblib/services/GBVMService.js';
import { ImageProcessingServices } from '../../basic.gblib/services/ImageProcessingServices.js';
import { ScheduleServices } from '../../basic.gblib/services/ScheduleServices.js';
import { SystemKeywords } from '../../basic.gblib/services/SystemKeywords.js';
import { WebAutomationServices } from '../../basic.gblib/services/WebAutomationServices.js';
import { GoogleChatDirectLine } from '../../google-chat.gblib/services/GoogleChatDirectLine.js';
import { AskDialogArgs } from '../../kb.gbapp/dialogs/AskDialog.js';
import { KBService } from '../../kb.gbapp/services/KBService.js';
import { SecService } from '../../security.gbapp/services/SecService.js';
import { WhatsappDirectLine } from '../../whatsapp.gblib/services/WhatsappDirectLine.js';
import { Messages } from '../strings.js';
import { GBConfigService } from './GBConfigService.js';
import { GBConversationalService } from './GBConversationalService.js';
import { GBDeployer } from './GBDeployer.js';
import { GBLogEx } from './GBLogEx.js';
import { GBSSR } from './GBSSR.js';

/**
 * Minimal service layer for a bot and encapsulation of BOT Framework calls.
 */
export class GBMinService {
  /**
   * Default General Bots User Interface package.
   */
  public static uiPackage = 'default.gbui';

  /**
   * Main core service attached to this bot service.
   */
  public core: IGBCoreService;

  /**
   * Reference to conversation services like receive and prompt text.
   */
  public conversationalService: IGBConversationalService;

  /**
   * Conversational administration services like publishing packages.
   */
  public adminService: IGBAdminService;

  /**
   * Deployent of packages and publishing related services.
   */
  public deployer: GBDeployer;

  bar1;
  static pidsConversation = {};

  /**
   * Static initialization of minimal instance.
   */
  constructor(
    core: IGBCoreService,
    conversationalService: IGBConversationalService,
    adminService: IGBAdminService,
    deployer: GBDeployer
  ) {
    this.core = core;
    this.conversationalService = conversationalService;
    this.adminService = adminService;
    this.deployer = deployer;
  }

  /**
   * Constructs a new minimal instance for each bot.
   */
  public async buildMin(instances: IGBInstance[]): Promise<GBMinInstance[]> {
    // Servers default UI on root address '/' if web enabled.

    if (process.env.DISABLE_WEB !== 'true') {
      // SSR processing and default.gbui access definition.

      GBServer.globals.server.get('/', async (req, res, next) => {
        await GBSSR.ssrFilter(req, res, next);
      });

      // Servers the bot information object via HTTP so clients can get
      // instance information stored on server.

      GBServer.globals.server.get('/instances/:botId', this.handleGetInstanceForClient.bind(this));
    }

    // Calls mountBot event to all bots.

    let i = 1;
    const minInstances = [];

    await CollectionUtil.asyncForEach(
      instances,
      (async instance => {
        try {
          GBLogEx.info(instance, `Mounting...`);
          const min = await this['mountBot'](instance);
          minInstances.push(min);
        } catch (error) {
          GBLogEx.error(instance, `Error mounting bot: ${error.message}\n${error.stack}`);
        }
      }).bind(this)
    );

    // Loads schedules.

    GBLogEx.info(0, `Loading SET SCHEDULE entries...`);
    const service = new ScheduleServices();
    await service.scheduleAll();

    GBLogEx.info(0, `All Bot service instances loaded.`);

    return minInstances;
  }

  public async startSimpleTest(min) {
    if (process.env.TEST_MESSAGE && min['isDefault']) {
      GBLogEx.info(min, `Starting auto test with '${process.env.TEST_MESSAGE}'.`);

      const client = await GBUtil.getDirectLineClient(min);

      const response = await client.apis.Conversations.Conversations_StartConversation();
      const conversationId = response.obj.conversationId;
      GBServer.globals.debugConversationId = conversationId;

      const steps = process.env.TEST_MESSAGE.split(';');
      const sec = new SecService();
      const user = await sec.ensureUser(min, 'testuser', 'testuser', '', 'test', 'testuser', null);

      const pid = GBVMService.createProcessInfo(user, min, 'api', null);
      await CollectionUtil.asyncForEach(steps, async step => {
        client.apis.Conversations.Conversations_PostActivity({
          conversationId: conversationId,
          activity: {
            textFormat: 'plain',
            text: step,
            pid: pid,
            type: 'message',
            from: {
              id: 'test',
              name: 'test'
            }
          }
        });

        await GBUtil.sleep(3000);
      });
    }
  }

  /**
   * Removes bot endpoint from web listeners and remove bot instance
   * from list of global server bot instances.
   */
  public async unmountBot(botId: string) {
    const url = `/api/messages/${botId}`;
    removeRoute(GBServer.globals.server, url);

    const uiUrl = `/${botId}`;
    removeRoute(GBServer.globals.server, uiUrl);

    GBServer.globals.minInstances = GBServer.globals.minInstances.filter(p => p.instance.botId !== botId);
  }

  /**
   * Mount the bot web site (default.gbui) secure domain.
   */
  public async loadDomain(min: GBMinInstance) {
    // TODO: https://github.com/GeneralBots/BotServer/issues/321
    const options = {
      passphrase: process.env.CERTIFICATE2_PASSPHRASE,
      pfx: await fs.readFile(process.env.CERTIFICATE2_PFX)
    };

    const domain = min.core.getParam(min.instance, 'Domain', null);
    if (domain) {
      GBServer.globals.server.get(domain, async (req, res, next) => {
        await GBSSR.ssrFilter(req, res, next);
      });
      GBLog.verbose(`Bot UI ${GBMinService.uiPackage} accessible at custom domain: ${domain}.`);
    }

    GBServer.globals.httpsServer.addContext(process.env.CERTIFICATE2_DOMAIN, options);
  }

  /**
   * Unmounts the bot web site (default.gbui) secure domain, if any.
   */
  public async unloadDomain(instance: IGBInstance) { }

  /**
   * Mount the instance by creating an BOT Framework bot object,
   * serving bot endpoint in several URL like WhatsApp endpoint, .gbkb assets,
   * installing all BASIC artifacts from .gbdialog and OAuth2.
   */
  public async mountBot(instance: IGBInstance) {
    // Build bot adapter.

    const { min, adapter, conversationState } = await this.buildBotAdapter(
      instance,
      GBServer.globals.sysPackages,
      GBServer.globals.appPackages
    );

    // https://github.com/GeneralBots/BotServer/issues/286
    // min['groupCache'] = await KBService.getGroupReplies(instance.instanceId);

    min['isDefault'] = GBServer.globals.minInstances.length === 0;

    GBServer.globals.minInstances.push(min);
    const user = null; // No user context.

    await GBVMService.loadConnections(min);

    // Install per bot deployed packages.

    let packagePath = urlJoin(`work`, GBUtil.getGBAIPath(min.botId, 'gbdialog'));
    if (await GBUtil.exists(packagePath)) {
      await this.deployer['deployPackage2'](min, user, packagePath);
    }
    packagePath = urlJoin(`work`, GBUtil.getGBAIPath(min.botId, 'gbapp'));
    if (await GBUtil.exists(packagePath)) {
      await this.deployer['deployPackage2'](min, user, packagePath);
    }
    packagePath = urlJoin(`work`, GBUtil.getGBAIPath(min.botId, 'gbtheme'));
    if (await GBUtil.exists(packagePath)) {
      await this.deployer['deployPackage2'](min, user, packagePath);
      await this.watchPackages(min, 'gbtheme');
    } else {
      await this.deployer['deployPackage2'](min, user, path.join('work', 'default.gbai', 'default.gbtheme'));
    }

    packagePath = urlJoin(`work`, GBUtil.getGBAIPath(min.botId, `gblib`));
    if (await GBUtil.exists(packagePath)) {
      await this.deployer['deployPackage2'](min, user, packagePath);
    }

    const gbai = GBUtil.getGBAIPath(min.botId);
    let dir = `work/${gbai}/cache`;
    const botId = gbai.replace(/\.[^/.]+$/, '');

    if (!(await GBUtil.exists(dir))) {
      mkdirp.sync(dir);
    }
    dir = `work/${gbai}/profile`;
    if (!(await GBUtil.exists(dir))) {
      mkdirp.sync(dir);
    }
    dir = `work/${gbai}/uploads`;
    if (!(await GBUtil.exists(dir))) {
      mkdirp.sync(dir);
    }

    dir = `work/${gbai}/${botId}.gbkb`;
    if (!(await GBUtil.exists(dir))) {
      mkdirp.sync(dir);
    }
    await this.watchPackages(min, 'gbkb');

    dir = `work/${gbai}/${botId}.gbkb/docs-vectorized`;
    if (!(await GBUtil.exists(dir))) {
      mkdirp.sync(dir);
    }

    dir = `work/${gbai}/${botId}.gbdialog`;
    if (!(await GBUtil.exists(dir))) {
      mkdirp.sync(dir);
    }
    await this.watchPackages(min, 'gbdialog');

    dir = `work/${gbai}/${botId}.gbot`;
    if (!(await GBUtil.exists(dir))) {
      mkdirp.sync(dir);
    }
    await this.watchPackages(min, 'gbot');

    dir = `work/${gbai}/${botId}.gbui`;
    if (!(await GBUtil.exists(dir))) {
      mkdirp.sync(dir);
    }

    dir = `work/${gbai}/users`;
    if (!(await GBUtil.exists(dir))) {
      mkdirp.sync(dir);
    }

    // Calls the loadBot context.activity for all packages.

    await this.invokeLoadBot(min.appPackages, GBServer.globals.sysPackages, min);
    const receiver = async (req, res) => {
      let path = /(http[s]?:\/\/)?([^\/\s]+\/)(.*)/gi;
      const botId = req.url.substr(req.url.lastIndexOf('/') + 1);

      const min = GBServer.globals.minInstances.filter(p => p.instance.botId == botId)[0];

      await this.receiver(req, res, conversationState, min, GBServer.globals.appPackages);
    };
    let url = `/api/messages/${instance.botId}`;
    GBServer.globals.server.post(url, receiver);

    if (min['default']) {
      url = `/api/messages`;
      GBServer.globals.server.post(url, receiver);
    }

    GBServer.globals.server.get(url, (req, res) => {
      if (req.query['hub.mode'] === 'subscribe') {
        if (req.query['hub.verify_token'] === process.env.FACEBOOK_VERIFY_TOKEN) {
          const val = req.query['hub.challenge'];
          res.send(val);
        } else {
          GBLog.error('Failed to verify endpoint.');
          res.send('OK');
        }
      }
      res.end();
    });

    await this.ensureAPI();

    GBLog.verbose(`GeneralBots(${instance.engineName}) listening on: ${url}.`);

    // Generates MS Teams manifest.

    const manifest = `${instance.botId}-Teams.zip`;
    const packageTeams = urlJoin(`work`, GBUtil.getGBAIPath(instance.botId), manifest);
    if (!(await GBUtil.exists(packageTeams))) {
      const data = await this.deployer.getBotManifest(instance);
      await fs.writeFile(packageTeams, data);
    }

    // Serves individual URL for each bot user interface.

    if (process.env.DISABLE_WEB !== 'true') {
      const uiUrl = `/${instance.botId}`;

      GBServer.globals.server.get(uiUrl, async (req, res, next) => {
        await GBSSR.ssrFilter(req, res, next);
      });
      const uiUrlAlt = `/${instance.activationCode}`;
      GBServer.globals.server.get(uiUrlAlt, async (req, res, next) => {
        await GBSSR.ssrFilter(req, res, next);
      });

      GBLog.verbose(`Bot UI ${GBMinService.uiPackage} accessible at: ${uiUrl} and ${uiUrlAlt}.`);
    }

    // Clients get redirected here in order to create an OAuth authorize url and redirect them to AAD.
    // There they will authenticate and give their consent to allow this app access to
    // some resource they own.

    this.handleOAuthRequests(GBServer.globals.server, min);

    // After consent is granted AAD redirects here.  The ADAL library
    // is invoked via the AuthenticationContext and retrieves an
    // access token that can be used to access the user owned resource.

    this.handleOAuthTokenRequests(GBServer.globals.server, min, instance);

    // Provides checking of instance health.

    this.createCheckHealthAddress(GBServer.globals.server, min, min.instance);

    // Setups official handler for WhatsApp.

    GBServer.globals.server
      .all(`/${min.instance.botId}/whatsapp`, async (req, res) => {

        const status = req.body?.entry?.[0]?.changes?.[0]?.value?.statuses?.[0];

        if (status) {
          GBLogEx.verbose(min, `WhatsApp: ${status.recipient_id} ${status.status}`);
          return;
        }

        if (req.query['hub.mode'] === 'subscribe') {
          const val = req.query['hub.verify_token'];
          const challenge = (min.core['getParam'] as any)(min.instance, `Meta Challenge`, null, true);

          if (challenge && val === challenge) {
            res.send(req.query['hub.challenge']);
            res.status(200);
            GBLogEx.info(min, `Meta callback OK. ${JSON.stringify(req.query)}`);
          } else {
            res.status(401);
          }
          res.end();

          return;
        }

        let whatsAppDirectLine = min.whatsAppDirectLine;

        // Not meta, multiples bots on root bot.

        if (!req.body.object) {
          const to = req.body.To.replace(/whatsapp\:\+/gi, '');
          whatsAppDirectLine = WhatsappDirectLine.botsByNumber[to];
        }

        if (whatsAppDirectLine) {
          await whatsAppDirectLine.WhatsAppCallback(req, res, whatsAppDirectLine.botId);
        }
      })
      .bind(min);

    GBDeployer.mountGBKBAssets(`${botId}.gbkb`, botId, `${botId}.gbkb`);

    return min;
  }

  public static getProviderName(req: any, res: any) {
    if (!res) {
      return 'GeneralBots';
    }

    if (req.body.entry) {
      return 'meta';
    }

    if (req.body?.AccountSid) {
      return 'official';
    }
    return req.body.phone_id ? 'maytapi' : 'chatapi';
  }

  /**
   * Creates a listener that can be used by external monitors to check
   * bot instance health.
   */
  private createCheckHealthAddress(server: any, min: GBMinInstance, instance: IGBInstance) {
    server.get(`/${min.instance.botId}/check`, async (req, res) => {
      try {
        // Performs the checking of WhatsApp API if enabled for this instance.

        if (min.whatsAppDirectLine != undefined && instance.whatsappServiceKey !== null) {
          if (!(await min.whatsAppDirectLine.check(min))) {
            const error = `WhatsApp API lost connection for: ${min.botId}.`;
            GBLog.error(error);
            res.status(500).send(error);

            return;
          }
        }

        // GB is OK, so 200.

        res.status(200).send(`General Bot ${min.botId} is healthly.`);
      } catch (error) {
        // GB is not OK, 500 and detail the information on response content.

        GBLog.error(error);
        res.status(500).send(error.toString());
      }
    });
  }

  /**
   * Handle OAuth2 web service calls for token requests
   * on https://<gbhost>/<BotId>/token URL.
   */
  private handleOAuthTokenRequests(server: any, min: GBMinInstance, instance: IGBInstance) {
    server.get(`/${min.instance.botId}/token`, async (req, res) => {
      let tokenName = req.query['value'];
      if (!tokenName) {
        tokenName = '';
      }

      // Checks request state by reading AntiCSRFAttackState from GB Admin infrastructure.

      const state = await min.adminService.getValue(instance.instanceId, `${tokenName}AntiCSRFAttackState`);
      if (req.query.state !== state) {
        const msg = 'WARNING: state field was not provided as anti-CSRF token';
        GBLog.error(msg);
        throw new Error(msg);
      }

      const clientId = min.core.getParam<string>(min.instance, `${tokenName} Client ID`, null);
      const clientSecret = min.core.getParam<string>(min.instance, `${tokenName} Client Secret`, null);
      const host = min.core.getParam<string>(min.instance, `${tokenName} Host`, null);
      const tenant = min.core.getParam<string>(min.instance, `${tokenName} Tenant`, null);

      if (tokenName) {
        const code = req?.query?.code;

        let url = urlJoin(host, tenant, 'oauth/token');
        let buff = new Buffer(`${clientId}:${clientSecret}`);
        const base64 = buff.toString('base64');

        const options = {
          method: 'POST',
          headers: {
            Accept: '1.0',
            Authorization: `Basic ${base64}`,
            'Content-Type': 'application/x-www-form-urlencoded'
          },
          body: new URLSearchParams({
            grant_type: 'authorization_code',
            code: code
          })
        };
        const result = await fetch(url, options);

        if (result.status != 200) {
          throw new Error(`handleOAuthTokenRequests error: ${result.status}: ${result.statusText}.`);
        }

        const text = await result.text();
        const token = JSON.parse(text);

        // Saves token to the database.

        await this.adminService.setValue(
          instance.instanceId,
          `${tokenName}accessToken`,
          token['accessToken'] ? token['accessToken'] : token['access_token']
        );
        await this.adminService.setValue(
          instance.instanceId,
          `${tokenName}refreshToken`,
          token['refreshToken'] ? token['refreshToken'] : token['refresh_token']
        );

        await this.adminService.setValue(
          instance.instanceId,
          `${tokenName}expiresOn`,
          token['expiresOn']
            ? token['expiresOn'].toString()
            : new Date(Date.now() + token['expires_in'] * 1000).toString()
        );
        await this.adminService.setValue(instance.instanceId, `${tokenName}AntiCSRFAttackState`, null);
      } else {
        const authenticationContext = new AuthenticationContext.AuthenticationContext(
          urlJoin(
            tokenName ? host : min.instance.authenticatorAuthorityHostUrl,
            tokenName ? tenant : min.instance.authenticatorTenant
          )
        );
        const resource = 'https://graph.microsoft.com';

        // Calls MSFT to get token.

        authenticationContext.acquireTokenWithAuthorizationCode(
          req.query.code,
          urlJoin(process.env.BOT_URL, min.instance.botId, '/token'),
          resource,
          tokenName ? clientId : instance.marketplaceId,
          tokenName ? clientSecret : instance.marketplacePassword,
          async (err, token) => {
            if (err) {
              const msg = `handleOAuthTokenRequests: Error acquiring token: ${err}`;

              GBLog.error(msg);
              res.send(msg);
            } else {
              // Saves token to the database.

              await this.adminService.setValue(instance.instanceId, `${tokenName}accessToken`, token['accessToken']);
              await this.adminService.setValue(instance.instanceId, `${tokenName}refreshToken`, token['refreshToken']);
              await this.adminService.setValue(
                instance.instanceId,
                `${tokenName}expiresOn`,
                token['expiresOn'].toString()
              );
              await this.adminService.setValue(instance.instanceId, `${tokenName}AntiCSRFAttackState`, null);
            }
          }
        );
      }
      // Inform the home for default .gbui after finishing token retrival.

      res.redirect(process.env.BOT_URL);
    });
  }

  /**
   * Handle OAuth2 web service calls for authorization requests
   * on https://<gbhost>/<BotId>/auth URL.
   */
  private handleOAuthRequests(server: any, min: GBMinInstance) {
    server.get(`/${min.instance.botId}/auth`, (req, res) => {
      let authorizationUrl = urlJoin(
        min.instance.authenticatorAuthorityHostUrl,
        min.instance.authenticatorTenant,
        '/oauth2/authorize'
      );
      authorizationUrl = `${authorizationUrl}?response_type=code&client_id=${min.instance.marketplaceId
        }&redirect_uri=${urlJoin(process.env.BOT_URL, min.instance.botId, 'token')}`;
      GBLogEx.info(min, `HandleOAuthRequests: ${authorizationUrl}.`);
      res.redirect(authorizationUrl);
    });
  }

  /**
   * Returns the instance object to clients requesting bot info.
   */
  private async handleGetInstanceForClient(req: any, res: any) {
    // Translates the requested botId.

    let botId = req.params.botId;
    if (botId === '[default]' || botId === undefined) {
      botId = GBConfigService.get('BOT_ID');
    }

    // Loads by the botId itself or by the activationCode field.

    let instance = await this.core.loadInstanceByBotId(botId);
    if (instance === null) {
      instance = await this.core.loadInstanceByActivationCode(botId);
    }

    if (instance !== null) {
      // Gets the webchat token, speech token and theme.

      const speechToken = instance.speechKey != undefined ? await this.getSTSToken(instance) : null;
      let theme = instance.theme;

      // Sends all information to the .gbui web client.

      if (!theme) {
        theme = `default.gbtheme`;
      }

      let logo = this.core.getParam(instance, 'Logo', null);

      logo = logo ? urlJoin(instance.botId, 'cache', logo) : 'images/logo-gb.png';

      let config = {
        instanceId: instance.instanceId,
        botId: botId,
        theme: theme,
        speechToken: speechToken,
        authenticatorTenant: instance.authenticatorTenant,
        authenticatorClientId: instance.marketplaceId,
        paramLogoImageUrl: this.core.getParam(instance, 'Logo Image Url', null),
        paramLogoImageAlt: this.core.getParam(instance, 'Logo Image Alt', null),
        paramLogoImageWidth: this.core.getParam(instance, 'Logo Image Width', null),
        paramLogoImageHeight: this.core.getParam(instance, 'Logo Image Height', null),
        paramLogoImageType: this.core.getParam(instance, 'Logo Image Type', null),
        logo: logo,
        color1: this.core.getParam(instance, 'Color1', null),
        color2: this.core.getParam(instance, 'Color2', null)
      };

      if (!GBConfigService.get('STORAGE_NAME')) {
        config['domain'] = `http://localhost:${GBConfigService.get('PORT')}/directline/${botId}`;
      } else {
        const webchatTokenContainer = await this.getWebchatToken(instance);
        config['conversationId'] = webchatTokenContainer.conversationId;
        config['webchatToken'] = webchatTokenContainer.token;
      }

      res.send(JSON.stringify(config));
    } else {
      const error = `Instance not found while retrieving from .gbui web client: ${botId}.`;
      res.sendStatus(error);
      GBLog.error(error);
    }
  }

  /**
   * Gets Webchat token from Bot Service.
   */
  private async getWebchatToken(instance: any) {
    const url = 'https://directline.botframework.com/v3/directline/tokens/generate';
    const options = {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${instance.webchatKey}`
      }
    };

    try {
      const res = await fetch(url, options);

      return await res.json();
    } catch (error) {
      const msg = `[botId:${instance.botId}] Error calling Direct Line to generate a token for Web control: ${error}.`;

      return Promise.reject(new Error(msg));
    }
  }

  /**
   * Gets a Speech to Text / Text to Speech token from the provider.
   */
  private async getSTSToken(instance: any) {
    const options = {
      method: 'POST',
      headers: {
        'Ocp-Apim-Subscription-Key': instance.speechKey
      }
    };
    const url = urlJoin(instance.speechEndpoint, '/sts/v1.0/issueToken');
    try {
      const res = await fetch(url, options);
      return res.text();
    } catch (error) {
      const msg = `Error calling Speech to Text client. Error is: ${error}.`;

      return Promise.reject(new Error(msg));
    }
  }

  /**
   * Builds the BOT Framework & GB infrastructures.
   */
  private async buildBotAdapter(instance: any, sysPackages: IGBPackage[], appPackages: IGBPackage[]) {
    // MSFT stuff.

    let config = {
      appId: instance.marketplaceId ? instance.marketplaceId : GBConfigService.get('MARKETPLACE_ID'),
      appPassword: instance.marketplacePassword
        ? instance.marketplacePassword
        : GBConfigService.get('MARKETPLACE_SECRET')
    };
    if (!GBConfigService.get('STORAGE_NAME')) {
      startRouter(GBServer.globals.server, instance.botId);
      config['clientOptions'] = { baseUri: `http://localhost:${GBConfigService.get('PORT')}` };
    }

    const adapter = new BotFrameworkAdapter(config);
    const storage = new MemoryStorage();
    const conversationState = new ConversationState(storage);
    const userState = new UserState(storage);
    adapter.use(new AutoSaveStateMiddleware(conversationState, userState));
    MicrosoftAppCredentials.trustServiceUrl(
      'https://directline.botframework.com',
      new Date(new Date().setFullYear(new Date().getFullYear() + 10))
    );

    // The minimal bot is built here.

    const min = new GBMinInstance();

    // Setups default BOT Framework dialogs.

    min.userProfile = conversationState.createProperty('userProfile');
    const dialogState = conversationState.createProperty('dialogState');

    min.dialogs = new DialogSet(dialogState);
    min.dialogs.add(new TextPrompt('textPrompt'));
    min.dialogs.add(new AttachmentPrompt('attachmentPrompt'));

    min.dialogs.add(new ConfirmPrompt('confirmPrompt'));
    if (process.env.ENABLE_AUTH) {
      min.dialogs.add(
        new OAuthPrompt('oAuthPrompt', {
          connectionName: 'OAuth2',
          text: 'Please sign in to General Bots.',
          title: 'Sign in',
          timeout: 300000
        })
      );
    }

    min.botId = instance.botId;
    min.bot = adapter;
    min.userState = userState;
    min.core = this.core;
    min.conversationalService = this.conversationalService;
    min.adminService = this.adminService;
    min.deployService = this.deployer;
    min.kbService = new KBService(this.core.sequelize);
    min.instance = instance;
    min.cbMap = {};
    min.scriptMap = {};
    min.sandBoxMap = {};
    min['scheduleMap'] = {};
    min['conversationWelcomed'] = {};
    if (await min.core.getParam(min.instance, 'Answer Mode', null)) {
      const gbkbPath = GBUtil.getGBAIPath(min.botId, 'gbkb');
      min['vectorStorePath'] = path.join('work', gbkbPath, 'docs-vectorized');
      min['vectorStore'] = await this.deployer.loadOrCreateEmptyVectorStore(min);
    }
    min['apiConversations'] = {};
    min.packages = sysPackages;

    // NLP Manager.

    const manager = new NlpManager({ languages: ['pt'], forceNER: true });
    min['nerEngine'] = manager;

    if (!GBServer.globals.minBoot.botId) {
      GBServer.globals.minBoot = min;
      GBServer.globals.minBoot.instance.marketplaceId = GBConfigService.get('MARKETPLACE_ID');
      GBServer.globals.minBoot.instance.marketplacePassword = GBConfigService.get('MARKETPLACE_SECRET');
    }

    if (min.instance.facebookWorkplaceVerifyToken) {
      min['fbAdapter'] = new FacebookAdapter({
        verify_token: min.instance.facebookWorkplaceVerifyToken,
        app_secret: min.instance.facebookWorkplaceAppSecret,
        access_token: min.instance.facebookWorkplaceAccessToken
      });
    }

    min.appPackages = await this.core['getApplicationsByInstanceId'](appPackages, min.instance.instanceId);

    // Creates a hub of services available in .gbapps.

    await CollectionUtil.asyncForEach(min.appPackages, async (e: IGBPackage) => {
      let services: ConcatArray<never>;
      if ((services = await e.onExchangeData(min, 'getServices', null))) {
        min.gbappServices = { ...min.gbappServices, ...services };
      }
    });

    if (min.instance.googlePrivateKey) {
      min['googleDirectLine'] = new GoogleChatDirectLine(
        min,
        min.botId,
        min.instance.googleBotKey,
        min.instance.googleChatSubscriptionName,
        min.instance.googleChatApiKey,
        min.instance.googleClientEmail,
        min.instance.googlePrivateKey.replace(/\\n/gm, '\n'),
        min.instance.googleProjectId
      );
      await min['googleDirectLine'].setup(true);
    }

    const group = min.core.getParam<string>(min.instance, 'WhatsApp Group ID', null);

    WhatsappDirectLine.botGroups[min.botId] = group;

    const minBoot = GBServer.globals.minBoot as any;

    // If there is WhatsApp configuration specified, initialize
    // infrastructure objects.

    if (min.instance.whatsappServiceKey) {
      min.whatsAppDirectLine = new WhatsappDirectLine(
        min,
        min.botId,
        min.instance.webchatKey,
        min.instance.whatsappServiceKey,
        min.instance.whatsappServiceNumber,
        min.instance.whatsappServiceUrl,
        group
      );

      await min.whatsAppDirectLine.setup(true);
    } else {
      if (min !== minBoot && minBoot.instance.whatsappServiceKey && min.instance.webchatKey) {
        min.whatsAppDirectLine = new WhatsappDirectLine(
          min,
          min.botId,
          min.instance.webchatKey,
          minBoot.instance.whatsappServiceKey,
          minBoot.instance.whatsappServiceNumber,
          minBoot.instance.whatsappServiceUrl,
          group
        );
        await min.whatsAppDirectLine.setup(false);
      }
    }

    // Builds bot numbers map in WhatsAppDirectLine globals.

    let botNumber = min.core.getParam<string>(min.instance, 'Bot Number', null);
    if (botNumber) {
      WhatsappDirectLine.botsByNumber[botNumber] = min.whatsAppDirectLine;
    }

    min['default'] = min === minBoot;

    return { min, adapter, conversationState };
  }

  /**
   * Performs calling of loadBot event in all .gbapps.
   */
  private async invokeLoadBot(appPackages: IGBPackage[], sysPackages: IGBPackage[], min: GBMinInstance) {
    // Calls loadBot event in all .gbapp packages.

    await CollectionUtil.asyncForEach(sysPackages, async p => {
      p.sysPackages = sysPackages;
      if (p.getDialogs !== undefined) {
        const dialogs = await p.getDialogs(min);
        if (dialogs !== undefined) {
          dialogs.forEach(dialog => {
            min.dialogs.add(new WaterfallDialog(dialog.id, dialog.waterfall));
          });
        }
      }

      await p.loadBot(min);
    });

    // Adds all dialogs from .gbapps into global dialo list for this minimal instance.

    await CollectionUtil.asyncForEach(appPackages, async p => {
      p.sysPackages = sysPackages;
      await p.loadBot(min);
      if (p.getDialogs !== undefined) {
        const dialogs = await p.getDialogs(min);
        if (dialogs !== undefined) {
          dialogs.forEach(dialog => {
            min.dialogs.add(new WaterfallDialog(dialog.id, dialog.waterfall));
          });
        }
      }
    });
  }

  // https://github.com/GeneralBots/BotServer/issues/313
  public static userMobile(step) {
    let mobile = WhatsappDirectLine.mobiles[step.context.activity.conversation.id];

    if (!mobile && step) {
      return step.context.activity.from.id;
    }

    return mobile;
  }

  /**
   * BOT Framework web service hook method.
   */
  private async receiver(
    req: any,
    res: any,
    conversationState: ConversationState,
    min: GBMinInstance,
    appPackages: any[]
  ) {
    // Uses standard or Facebook Adapter.

    let adapter = min.bot;
    if (req.body.object) {
      req['rawBody'] = JSON.stringify(req.body);
      adapter = min['fbAdapter'];
    }

    // Unifies channel detection.  Unmarshalls group information.

    req.body.channelId = req.body?.from?.channelIdEx === 'whatsapp' ? 'omnichannel' : req.body.channelId;
    req.body.group = req.body?.from?.group;

    // Default activity processing and handler.

    const handler = async context => {

      // Handle activity text issues.

      if (!context.activity.text) {
        context.activity.text = '';
      }
      context.activity.text = context.activity.text.replace(/\@General Bots Online /gi, '');

      // Get loaded user state

      const step = await min.dialogs.createContext(context);
      step.context.activity.locale = 'pt-BR';


      const sec = new SecService();
      let member = context.activity.recipient;
      
      
      if (process.env.STORAGE_NAME){
        member  = context.activity.from;
      }
      let user = await sec.ensureUser(min, member.id, member.name, '', 'web', member.name, null);
      const userId = user.userId;
      const params = user.params ? JSON.parse(user.params) : {};

      try {
        const conversationReference = JSON.stringify(TurnContext.getConversationReference(context.activity));
        user = await sec.updateConversationReferenceById(user.userId, conversationReference);

        // First time processing.

        if (!params.loaded) {
          if (step.context.activity.channelId !== 'msteams') {
            await min.conversationalService.sendEvent(min, step, 'loadInstance', {});
          }

          // Default params.

          await sec.setParam(userId, 'loaded', true);
          await sec.setParam(userId, 'subjects', '[]');
          await sec.setParam(userId, 'cb', null);
          await sec.setParam(userId, 'welcomed', 'false');
          await sec.setParam(userId, 'maxLines', 100);
          await sec.setParam(userId, 'translatorOn', true);
          await sec.setParam(userId, 'wholeWord', true);
          await sec.setParam(userId, 'theme', 'white');
          await sec.setParam(userId, 'maxColumns', 40);

          // This same event is dispatched either to all participants
          // including the bot, that is filtered bellow.

          if (context.activity.from.id !== min.botId) {
            // Creates a new row in user table if it does not exists.
            if (process.env.PRIVACY_STORE_MESSAGES === 'true') {
              // Stores conversation associated to the user to group each message.

              const analytics = new AnalyticsService();
              await analytics.createConversation(user);
            }
          }

          await sec.updateConversationReferenceById(userId, conversationReference);

          if (step.context.activity.channelId !== 'msteams') {
            const service = new KBService(min.core.sequelize);
            const data = await service.getFaqBySubjectArray(min.instance.instanceId, 'faq', undefined);
            await min.conversationalService.sendEvent(min, step, 'play', {
              playerType: 'bullet',
              data: data.slice(0, 10)
            });
          }
        }

        let conversationId = step.context.activity.conversation.id;

        let pid = GBMinService.pidsConversation[conversationId];

        if (!pid) {

          pid = step.context.activity['pid'];
          if (!pid) {
            pid = WhatsappDirectLine.pidByNumber[context.activity.from.id];
            if (!pid) {
              pid = GBVMService.createProcessInfo(user, min, step.context.activity.channelId, null, step);
            }
          }
        }
        GBMinService.pidsConversation[conversationId] = pid;
        step.context.activity['pid'] = pid;

        const notes = min.core.getParam(min.instance, 'Notes', null);
        if (await this.handleUploads(min, step, user, params, notes != null)) {
          return;

        }

        // Required for MSTEAMS handling of persisted conversations.

        if (step.context.activity.channelId === 'msteams') {
          if (step.context.activity.attachments && step.context.activity.attachments.length > 1) {
            const file = context.activity.attachments[0];
            const credentials = new MicrosoftAppCredentials(
              min.instance.marketplaceId,
              min.instance.marketplacePassword
            );
            const botToken = await credentials.getToken();
            const headers = { Authorization: `Bearer ${botToken}` };
            const t = new SystemKeywords();
            const data = await t.getByHttp({
              pid: 0,
              url: file.contentUrl,
              headers,
              username: null,
              ps: null,
              qs: null
            });
            const packagePath = GBUtil.getGBAIPath(min.botId);
            const folder = `work/${path}/cache`;
            const filename = `${GBAdminService.generateUuid()}.png`;

            await fs.writeFile(urlJoin(folder, filename), data);
            step.context.activity.text = urlJoin(
              GBServer.globals.publicAddress,
              `${min.instance.botId}`,
              'cache',
              filename
            );
          }

          if (!(await sec.getParam(user, 'welcomed'))) {
            const startDialog = min.core.getParam(min.instance, 'Start Dialog', null);
            if (startDialog) {
              await sec.setParam(userId, 'welcomed', 'true');
              GBLogEx.info(
                min,
                `Auto start (teams) dialog is now being called: ${startDialog} for ${min.instance.botId}...`
              );

              await GBVMService.callVM(startDialog.toLowerCase(), min, step, 0);
            }
          }
        }

        // Answer to specific BOT Framework event conversationUpdate to auto start dialogs.
        // Skips if the bot is talking.

        const startDialog = min.core.getParam(min.instance, 'Start Dialog', null);

        if (context.activity.type === 'installationUpdate') {
          GBLogEx.info(min, `Bot installed on Teams.`);
        } else if (context.activity.type === 'conversationUpdate' &&
          context.activity.membersAdded.length > 0) {
          // Check if a bot or a human participant is being added to the conversation.

          const member = context.activity.membersAdded[0];
          if (context.activity.membersAdded[0].id === context.activity.recipient.id) {
            GBLogEx.info(min, `Bot added to conversation, starting chat...`);

            // Calls onNewSession event on each .gbapp package.

            await CollectionUtil.asyncForEach(appPackages, async e => {
              await e.onNewSession(min, step);
            });

            // Auto starts dialogs if any is specified.

            if (!startDialog && !(await sec.getParam(user, 'welcomed'))) {
              // Otherwise, calls / (root) to default welcome users.

              await step.beginDialog('/');
            } else {
              if (
                !GBMinService.userMobile(step) &&
                !min['conversationWelcomed'][step.context.activity.conversation.id]
              ) {

                const pid = GBVMService.createProcessInfo(user, min, step.context.activity.channelId, null, step);
                step.context.activity['pid'] = pid;

                min['conversationWelcomed'][step.context.activity.conversation.id] = true;

                GBLogEx.info(
                  min,
                  `Auto start (web 1) dialog is now being called: ${startDialog} for ${min.instance.instanceId}...`
                );
                await GBVMService.callVM(startDialog.toLowerCase(), min, step, pid);
              }
            }
          } else {
            GBLogEx.info(min, `Person added to conversation: ${member.name}`);

            return;
          }
        } else if (context.activity.type === 'message') {



          // Required for F0 handling of persisted conversations.

          GBLogEx.info(
            min,
            `Human: pid:${pid} ${context.activity.from.id} ${GBUtil.toYAML(WhatsappDirectLine.pidByNumber)} ${context.activity.text} (type: ${context.activity.type}, name: ${context.activity.name}, channelId: ${context.activity.channelId})`
          );


          // Processes messages activities.

          await this.processMessageActivity(context, min, step, pid);
        } else if (context.activity.type === 'event') {
          // Processes events activities.

          await this.processEventActivity(min, user, context, step);
        }
      } catch (error) {
        GBLog.error(`Receiver: ${GBUtil.toYAML(error)}`);

        await min.conversationalService.sendText(
          min,
          step,
          Messages[step.context.activity.locale].very_sorry_about_error
        );

        await step.beginDialog('/ask', { isReturning: true });
      }
    };

    try {
      if (!GBConfigService.get('STORAGE_NAME')) {
        const context = adapter['createContext'](req);
        context['_activity'] = context.activity.body;
        await handler(context);

        // Return status
        res.status(200);

        res.end();
      } else {
        await adapter['processActivity'](req, res, handler);
      }
    } catch (error) {
      if (error.code === 401) {
        GBLog.error('Calling processActivity due to Signing Key could not be retrieved error.');
        await adapter['processActivity'](req, res, handler);
      } else {
        GBLog.error(`Error processing activity: ${GBUtil.toYAML(error)}`);
        throw error;
      }
    }
  }

  /**
   * Called to handle all event sent by .gbui clients.
   */
  private async processEventActivity(min, user, context, step: GBDialogStep) {
    const pid = step.context.activity['pid'];
    if (context.activity.name === 'whoAmI') {
      await step.beginDialog('/whoAmI');
    } else if (context.activity.name === 'showSubjects') {
      await step.beginDialog('/menu', undefined);
    } else if (context.activity.name === 'giveFeedback') {
      await step.beginDialog('/feedback', {
        fromMenu: true
      });
    } else if (context.activity.name === 'showFAQ') {
      await step.beginDialog('/faq');
    } else if (context.activity.name === 'answerEvent') {
      await step.beginDialog('/answerEvent', <AskDialogArgs>{
        questionId: context.activity.data,
        fromFaq: true
      });
    } else if (context.activity.name === 'quality') {
      await step.beginDialog('/quality', {
        score: context.activity.data
      });
    } else if (context.activity.name === 'startGB') {
      const startDialog = min.core.getParam(min.instance, 'Start Dialog', null);
      if (startDialog && !min['conversationWelcomed'][step.context.activity.conversation.id]) {
        user.welcomed = true;
        GBLogEx.info(
          min,
          `Auto start (web 2) dialog is now being called: ${startDialog} for ${min.instance.instanceId}...`
        );
        await GBVMService.callVM(startDialog.toLowerCase(), min, step, pid);
      }
    } else if (context.activity.name === 'updateToken') {
      const token = context.activity.data;
      await step.beginDialog('/adminUpdateToken', { token: token });
    } else {
      await step.continueDialog();
    }
  }

  /**
   * Private handler which receives the Attachment and persists to disk.
   * during a HEAR attachment AS FILE upload.
   */
  // ...

  private static async downloadAttachmentAndWrite(attachment) {
    const url = attachment.contentUrl;
    const localFolder = 'work';
    const packagePath = GBUtil.getGBAIPath(this['min'].botId);
    const localFileName = path.join(localFolder, packagePath, 'cache', attachment.name);

    let buffer;

    if (url.startsWith('data:')) {
      const base64Data = url.split(';base64,')[1];
      buffer = Buffer.from(base64Data, 'base64');
    } else {
      const options = {
        method: 'GET',
        encoding: 'binary'
      };
      const res = await fetch(url, options);
      buffer = arrayBufferToBuffer(await res.arrayBuffer());
    }

    await fs.writeFile(localFileName, buffer);

    return {
      name: attachment.name,
      filename: localFileName,
      url: url,
      data: buffer
    };
  }

  /**
   *
   * Checks for global exit kewywords cancelling any active dialogs.
   *
   * */
  public static isGlobalQuitUtterance(locale, utterance) {
    return utterance.match(Messages.global_quit);
  }

  private async handleUploads(min, step, user, params, autoSave) {
    // Prepare Promises to download each attachment and then execute each Promise.
    if (
      step.context.activity.attachments &&
      step.context.activity.attachments[0] &&
      step.context.activity.attachments[0].contentType != 'text/html'
    ) {
      const promises = step.context.activity.attachments.map(
        GBMinService.downloadAttachmentAndWrite.bind({ min, user, params })
      );
      const successfulSaves = await Promise.all(promises);
      async function replyForReceivedAttachments(attachmentData) {
        if (attachmentData) {

          // In case of not having HEAR activated before, it is
          // a upload with no Dialog, so run Auto Save to .gbdrive.

          const t = new SystemKeywords();
          GBLogEx.info(min, `BASIC (${min.botId}): Upload2 done for ${attachmentData.filename}.`);
          const handle = WebAutomationServices.cyrb53({ pid: 0, str: min.botId + attachmentData.filename });
          let data = await fs.readFile(attachmentData.filename);

          const gbfile = {
            filename: path.join(process.env.PWD, attachmentData.filename),
            data: data,
            url: attachmentData.url,
            name: path.basename(attachmentData.filename)
          };

          GBServer.globals.files[handle] = gbfile;

          if (!min.cbMap[user.userId] && autoSave) {
            const result = await t['internalAutoSave']({ min: min, handle: handle });
            await min.conversationalService.sendText(
              min,
              step,
              `Seu arquivo ${gbfile.name} foi salvo no .gbdrive (${result.category}).`
            );

            return;
          } else {
            return gbfile;
          }
        } else {
          await this.sendActivity('Error uploading file. Please,start again.');
        }
      }
      const replyPromises = successfulSaves.map(replyForReceivedAttachments.bind(step.context));
      await Promise.all(replyPromises);
      if (successfulSaves.length > 0) {
        class GBFile {
          data: Buffer;
          filename: string;
          url: string;
          name: string;
        }

        const results = await successfulSaves.reduce(async (accum: GBFile[], item) => {
          const result: GBFile = {
            data: await fs.readFile(successfulSaves[0]['filename']),
            filename: successfulSaves[0]['filename'],
            name: successfulSaves[0]['name'],
            url: successfulSaves[0]['url'],
          };
          accum.push(result);
          return accum;
        }, []) as GBFile[];

        if (min.cbMap[user.userId] && min.cbMap[user.userId].promise == '!GBHEAR') {
          if (results.length > 1) {
            throw new Error('It is only possible to upload one file per message, right now.');
          }
          min.cbMap[user.userId].promise = results[0];
        }
      }
      return successfulSaves.length > 0;
    }
    return false;
  }

  /**
   * Called to handle all text messages sent and received by the bot.
   */
  private async processMessageActivity(context, min: GBMinInstance, step: GBDialogStep, pid) {
    const sec = new SecService();

    if (!context.activity.text) {
      context.activity.text = '';
    }

    // Removes <at>Bot Id</at> from MS Teams.

    context.activity.text = context.activity.text.replace(/\<at\>.*\<\/at\>\s/gi, '');

    let data = { query: context.activity.text };
    await CollectionUtil.asyncForEach(min.appPackages, async (e: IGBPackage) => {
      await e.onExchangeData(min, 'handleRawInput', data);
    });
    context.activity.text = data.query;

    // Additional clean up.

    context.activity.text = context.activity.text.trim();

    const member = context.activity.from;
    let memberId = null,
      email = null;

    // Processes e-mail from id in case of Teams messages.

    if (member.id.startsWith('29:')) {
      const token = await (min.adminService as any)['acquireElevatedToken'](min.instance.instanceId, false);

      const url = `https://graph.microsoft.com/v1.0/users/${context.activity.from.aadObjectId}`;
      const options = {
        method: 'GET',
        headers: {
          Authorization: `Bearer ${token}`
        }
      };

      try {
        const res = await fetch(url, options);
        const member = JSON.parse(await res.text());
        memberId = member.mail;
        email = member.mail;
      } catch (error) {
        throw `[botId:${min.instance.botId}] Error calling Teams to get user info: ${error}.`;
      }
    } else {
      memberId = member.id;
    }

    let user = await sec.ensureUser(min, memberId, member.name, '', 'web', member.name, email);

    const userId = user.userId;
    const params = user.params ? JSON.parse(user.params) : {};

    let message: GuaribasConversationMessage;
    if (process.env.PRIVACY_STORE_MESSAGES === 'true') {
      // Adds message to the analytics layer.

      const analytics = new AnalyticsService();

      if (user) {
        let conversation;
        if (!user.conversationId) {
          conversation = await analytics.createConversation(user);
          user.conversationId = conversation.Id;
        }

        message = await analytics.createMessage(
          min.instance.instanceId,
          user.conversationId,
          userId,
          context.activity.text
        );
      }
    }

    const conversationReference = JSON.stringify(TurnContext.getConversationReference(context.activity));
    await sec.updateConversationReferenceById(userId, conversationReference);

    if (GBMinService.userMobile(step)) {
      const startDialog = user.hearOnDialog ? user.hearOnDialog : min.core.getParam(min.instance, 'Start Dialog', null);

      if (
        startDialog &&
        startDialog !== '' &&
        !min['conversationWelcomed'][step.context.activity.conversation.id] &&
        !min['apiConversations'][pid] &&
        !step.context.activity['group']
      ) {
        await sec.setParam(userId, 'welcomed', 'true');
        min['conversationWelcomed'][step.context.activity.conversation.id] = true;
        GBLogEx.info(
          min,
          `Auto start (4) dialog is now being called: ${startDialog} for ${min.instance.instanceId}...`
        );
        await GBVMService.callVM(startDialog.toLowerCase(), min, step, pid);



      }
    }

    // Files in .gbdialog can be called directly by typing its name normalized into JS .

    const isVMCall = Object.keys(min.scriptMap).find(key => min.scriptMap[key] === context.activity.text) !== undefined;

    // TODO: Externalize intents for LLM.

    if (/create dialog|creative dialog|create a dialog|criar diálogo|criar diálogo/gi.test(context.activity.text)) {
      await step.beginDialog('/dialog');
    } else if (isVMCall) {
      await GBVMService.callVM(context.activity.text, min, step, pid);
    } else if (context.activity.text.charAt(0) === '/') {
      const text = context.activity.text;
      const parts = text.split(' ');
      const cmdOrDialogName = parts[0];
      parts.splice(0, 1);
      const args = parts.join(' ');
      if (cmdOrDialogName === '/start') {
        // Reset user.

        await min.conversationalService.sendEvent(min, step, 'loadInstance', {});
      } else if (cmdOrDialogName === '/call') {
        await GBVMService.callVM(args, min, step, pid);
      } else if (cmdOrDialogName === '/callsch') {
        await GBVMService.callVM(args, min, null, pid);
      } else if (cmdOrDialogName === '/calldbg') {
        await GBVMService.callVM(args, min, step, pid, true);
      } else {
        await step.beginDialog(cmdOrDialogName, { args: args });
      }
    } else if (GBMinService.isGlobalQuitUtterance(step.context.activity.locale, context.activity.text)) {
      await step.cancelAllDialogs();
      await min.conversationalService.sendText(min, step, Messages[step.context.activity.locale].canceled);
    } else if (context.activity.text === 'admin') {
      await step.beginDialog('/admin');
    } else if (context.activity.text.startsWith('{"title"')) {
      await step.beginDialog('/menu', JSON.parse(context.activity.text));
    } else if (
      !(await this.deployer.getStoragePackageByName(min.instance.instanceId, `${min.instance.botId}.gbkb`)) &&
      process.env.GBKB_ENABLE_AUTO_PUBLISH === 'true'
    ) {
      await min.conversationalService.sendText(
        min,
        step,
        `Oi, ainda não possuo pacotes de conhecimento publicados. Por favor, aguarde alguns segundos enquanto eu auto-publico alguns pacotes.`
      );
      await step.beginDialog('/publish', { confirm: true, firstTime: true });
    } else {
      // Removes unwanted chars in input text.

      step.context.activity['originalText'] = context.activity.text;
      const text = await GBConversationalService.handleText(min, user, step, context.activity.text);
      step.context.activity['originalText'];
      step.context.activity['text'] = text;

      // Checks for bad words on input text.

      const hasBadWord = wash.check(step.context.activity.locale, text);
      if (hasBadWord) {
        return await step.beginDialog('/pleaseNoBadWords');
      }

      if (user.agentMode === 'self') {
        const manualUser = await sec.getUserFromAgentSystemId(user.userSystemId);

        GBLogEx.info(min, `HUMAN AGENT (${user.userId}) TO USER ${manualUser.userSystemId}: ${text}`);

        const cmd = 'SEND FILE ';
        if (text.startsWith(cmd)) {
          const filename = text.substr(cmd.length);
          const message = await min.kbService.getAnswerTextByMediaName(min.instance.instanceId, filename);

          if (message === null) {
            GBLog.error(
              `File ${filename} not found in any .gbkb published. Check the name or publish again the associated .gbkb.`
            );
          } else {
            await min.conversationalService.sendMarkdownToMobile(min, null, manualUser.userSystemId, message);
          }
        } else {
          await min.whatsAppDirectLine.sendToDeviceEx(
            manualUser.userSystemId,
            `${manualUser.agentSystemId}: ${text}`,
            step.context.activity.locale,
            step.context.activity.conversation.id
          );
        }
      } else {
        if (min.cbMap[userId] && min.cbMap[userId].promise === '!GBHEAR') {
          min.cbMap[userId].promise = step.context.activity['originalText'];
        }

        // If there is a dialog in course, continue to the next step.
        else if (step.activeDialog !== undefined) {
          try {
            await step.continueDialog();
          } catch (error) {
            const msg = `ERROR: ${error.message} ${error.stack} ${error.error ? error.error.body : ''} ${error.error ? (error.error.stack ? error.error.stack : '') : ''
              }`;
            GBLog.error(msg);
            await min.conversationalService.sendText(
              min,
              step,
              Messages[step.context.activity.locale].very_sorry_about_error
            );
            await step.beginDialog('/ask', { isReturning: true });
          }
        } else {
          const startDialog = user.hearOnDialog
            ? user.hearOnDialog
            : min.core.getParam(min.instance, 'Start Dialog', null);

          if (text !== startDialog) {
            let nextDialog = null;
            let data = {
              query: text,
              step: step,
              notTranslatedQuery: context.activity.text,
              message: message ? message['dataValues'] : null,
              user: user ? user.dataValues : null
            };
            await CollectionUtil.asyncForEach(min.appPackages, async (e: IGBPackage) => {
              if (!nextDialog) {
                nextDialog = await e.onExchangeData(min, 'handleAnswer', data);
              }
            });
            data.step = null;
            GBLogEx.info(min, `/answer from processMessageActivity (nextDialog=${nextDialog}).`);
            await step.beginDialog(nextDialog ? nextDialog : '/answer', {
              data: data,
              query: text,
              user: user ? user.dataValues : null,
              message: message
            });
          }
        }
      }
    }
  }

  public async ensureAPI() {
    const mins = GBServer.globals.minInstances;

    function getRemoteId(ctx: Koa.Context) {
      return '1'; // Each bot has its own API.
    }

    const close = async () => {
      return new Promise(resolve => {
        if (GBServer.globals.server.apiServer) {
          GBServer.globals.server.apiServer.close(cb => {
            resolve(true);
          });
        } else {
          resolve(true);
          GBLogEx.info(0, 'Loading General Bots API...');
        }
      });
    };

    await close();

    let proxies = {};
    await CollectionUtil.asyncForEach(mins, async min => {
      let dialogs = {};

      await CollectionUtil.asyncForEach(Object.values(min.scriptMap), async script => {
        const api = min.core.getParam(min.instance, 'Server API', null);
        if (api) {
          dialogs[script] = async data => {
            let sec = new SecService();
            const user = await sec.ensureUser(
              min,
              data.userSystemId,
              data.userName ? data.userName : 'apiuser',
              '',
              'api',
              data.userSystemId,
              null
            );

            let pid = data?.pid;
            if (script === 'start') {
              pid = GBVMService.createProcessInfo(user, min, 'api', null);

              const client = await GBUtil.getDirectLineClient(min);
              const response = await client.apis.Conversations.Conversations_StartConversation({
                userSystemId: user.userSystemId,
                userName: user.userName,
                pid: pid
              });

              min['apiConversations'][pid] = { conversation: response.obj, client: client };
              min['conversationWelcomed'][response.obj.id] = true;
            }

            let ret = await GBVMService.callVM(script, min, null, pid, false, data);

            if (script === 'start') {
              ret = pid;
            }
            return ret;
          };
        }
      });

      const proxy = {
        dk: new DialogKeywords(),
        wa: new WebAutomationServices(),
        sys: new SystemKeywords(),
        dbg: new DebuggerService(),
        img: new ImageProcessingServices(),
        dialogs: dialogs
      };
      proxies[min.botId] = proxy;
    });

    const opts = {
      pingSendTimeout: null,
      keepAliveTimeout: null,
      listeners: {
        unsubscribed(subscriptions: number): void { },
        subscribed(subscriptions: number): void { },
        disconnected(remoteId: string, connections: number): void { },
        connected(remoteId: string, connections: number): void { },
        messageIn(...params): void {
          params.shift();
        },
        messageOut(...params): void {
          params.shift();
        }
      }
    };

    GBServer.globals.server.apiServer = createKoaHttpServer(GBVMService.API_PORT, getRemoteId, { prefix: `api/v3` });

    createRpcServer(proxies, GBServer.globals.server.apiServer, opts);
  }

  // Map to track recent changes with timestamps

  private recentChanges: Set<string> = new Set();
  private mutex: Mutex = new Mutex();

  public async watchPackages(min: GBMinInstance, packageType: string): Promise<void> {
    if (!GBConfigService.get('STORAGE_NAME')) {
      const packagePath = GBUtil.getGBAIPath(min.botId, packageType);
      const libraryPath = path.join(GBConfigService.get('STORAGE_LIBRARY'), packagePath);

      const watcher = chokidar.watch(libraryPath, {
        depth: 99 // Watch subdirectories
      });

      const handleFileChange = async (filePath: string) => {
        this.recentChanges.add(filePath);

        // Use mutex to ensure only one deployment runs at a time
        await this.mutex.runExclusive(async () => {
          if (this.recentChanges.size > 0) {
            try {
              const workFolder = path.join('work', packagePath);
              await this.deployer.deployPackage2(min, null, workFolder, true);
              GBLogEx.info(min, `Deployed: ${path.basename(workFolder)}.`);
            } catch (error) {
              GBLogEx.error(min, `Error deploying package: ${GBUtil.toYAML(error)}`);
            } finally {
              this.recentChanges.clear();
            }
          }
        });
      };

      // Watch for file changes
      watcher.on('change', filePath => {
        handleFileChange(filePath).catch(error => console.error('Error processing file change:', error));
      });
    }
  }
}
