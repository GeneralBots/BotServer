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
import cliProgress from 'cli-progress';
import { DialogSet, TextPrompt } from 'botbuilder-dialogs';
import express from 'express';
import SwaggerClient from 'swagger-client';
import removeRoute from 'express-remove-route';
import AuthenticationContext from 'adal-node';
import wash from 'washyourmouthoutwithsoap';
import { FacebookAdapter } from 'botbuilder-adapter-facebook';
import path from 'path';
import mkdirp from 'mkdirp';
import Fs from 'fs';
import arrayBufferToBuffer from 'arraybuffer-to-buffer';
import { NlpManager } from 'node-nlp';
import {
  AutoSaveStateMiddleware,
  BotFrameworkAdapter,
  ConversationState,
  MemoryStorage,
  TurnContext,
  UserState
} from 'botbuilder';
import { AttachmentPrompt, ConfirmPrompt, OAuthPrompt, WaterfallDialog } from 'botbuilder-dialogs';
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
import { CollectionUtil } from 'pragmatismo-io-framework';
import { MicrosoftAppCredentials } from 'botframework-connector';
import { GBServer } from '../../../src/app.js';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import { GuaribasConversationMessage } from '../../analytics.gblib/models/index.js';
import { AnalyticsService } from '../../analytics.gblib/services/AnalyticsService.js';
import { GBVMService } from '../../basic.gblib/services/GBVMService.js';
import { AskDialogArgs } from '../../kb.gbapp/dialogs/AskDialog.js';
import { KBService } from '../../kb.gbapp/services/KBService.js';
import { SecService } from '../../security.gbapp/services/SecService.js';
import { WhatsappDirectLine } from '../../whatsapp.gblib/services/WhatsappDirectLine.js';
import { Messages } from '../strings.js';
import { GBConfigService } from './GBConfigService.js';
import { GBConversationalService } from './GBConversationalService.js';
import { GBDeployer } from './GBDeployer.js';
import urlJoin from 'url-join';
import { GoogleChatDirectLine } from '../../google-chat.gblib/services/GoogleChatDirectLine.js';
import { SystemKeywords } from '../../basic.gblib/services/SystemKeywords.js';
import Path from 'path';
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
  public async buildMin(instances: IGBInstance[]) {
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

    if (instances.length > 1) {
      this.bar1 = new cliProgress.SingleBar(
        {
          format: '[{bar}] ({value}/{total}) Loading {botId} ...',
          barsize: 40,
          forceRedraw: true
        },
        cliProgress.Presets.rect
      );
      this.bar1.start(instances.length, i, { botId: 'Boot' });
    }

    const throttledPromiseAll = async promises => {
      const MAX_IN_PROCESS = 20;
      const results = new Array(promises.length);

      async function doBlock(startIndex) {
        // Shallow-copy a block of promises to work on
        const currBlock = promises.slice(startIndex, startIndex + MAX_IN_PROCESS);
        // Await the completion. If any fail, it will throw and that's good.
        const blockResults = await Promise.all(currBlock);
        // Assuming all succeeded, copy the results into the results array
        for (let ix = 0; ix < blockResults.length; ix++) {
          results[ix + startIndex] = blockResults[ix];
        }
      }

      for (let iBlock = 0; iBlock < promises.length; iBlock += MAX_IN_PROCESS) {
        await doBlock(iBlock);
      }
      return results;
    };

    await throttledPromiseAll(
      instances.map(
        (async instance => {
          try {
            await this['mountBot'](instance);

            if (this.bar1) {
              this.bar1.update(i++, { botId: instance.botId });
            }
          } catch (error) {
            GBLog.error(`Error mounting bot ${instance.botId}: ${error.message}\n${error.stack}`);
          }
        }).bind(this)
      )
    );
    if (this.bar1) {
      this.bar1.stop();
    }

    // // Loads schedules.
    // GBLog.info(`Preparing SET SCHEDULE dialog calls...`);

    // const service = new ScheduleServices();
    // await service.scheduleAll();

    GBLog.info(`All Bot instances loaded.`);
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
      pfx: Fs.readFileSync(process.env.CERTIFICATE2_PFX)
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
  public async unloadDomain(instance: IGBInstance) {}

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

    GBServer.globals.minInstances.push(min);
    const user = null; // No user context.

    await this.deployer['deployPackage2'](min, user, 'packages/default.gbtheme');

    // Install per bot deployed packages.

    let packagePath = `work/${min.botId}.gbai/${min.botId}.gbdialog`;
    if (Fs.existsSync(packagePath)) {
      await this.deployer['deployPackage2'](min, user, packagePath);
    }
    packagePath = `work/${min.botId}.gbai/${min.botId}.gbapp`;
    if (Fs.existsSync(packagePath)) {
      await this.deployer['deployPackage2'](min, user, packagePath);
    }
    packagePath = `work/${min.botId}.gbai/${min.botId}.gbtheme`;
    if (Fs.existsSync(packagePath)) {
      await this.deployer['deployPackage2'](min, user, packagePath);
    }
    packagePath = `work/${min.botId}.gbai/${min.botId}.gblib`;
    if (Fs.existsSync(packagePath)) {
      await this.deployer['deployPackage2'](min, user, packagePath);
    }

    let dir = `work/${min.botId}.gbai/cache`;

    if (!Fs.existsSync(dir)) {
      mkdirp.sync(dir);
    }
    dir = `work/${min.botId}.gbai/profile`;
    if (!Fs.existsSync(dir)) {
      mkdirp.sync(dir);
    }
    dir = `work/${min.botId}.gbai/uploads`;
    if (!Fs.existsSync(dir)) {
      mkdirp.sync(dir);
    }
    dir = `work/${min.botId}.gbai/${min.botId}.gbui`;
    if (!Fs.existsSync(dir)) {
      mkdirp.sync(dir);
    }

    // Loads Named Entity data for this bot.

    await KBService.RefreshNER(min);

    // Calls the loadBot context.activity for all packages.

    await this.invokeLoadBot(GBServer.globals.appPackages, GBServer.globals.sysPackages, min);

    // Serves individual URL for each bot conversational interface.

    const receiver = async (req, res) => {
      await this.receiver(req, res, conversationState, min, instance, GBServer.globals.appPackages);
    };
    const url = `/api/messages/${instance.botId}`;
    GBServer.globals.server.post(url, receiver);
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
    GBLog.verbose(`GeneralBots(${instance.engineName}) listening on: ${url}.`);

    // Test code.
    if (process.env.TEST_MESSAGE) {
      GBLog.info(`Starting auto test with '${process.env.TEST_MESSAGE}'.`);

      const client = await new SwaggerClient({
        spec: JSON.parse(Fs.readFileSync('directline-3.0.json', 'utf8')),
        requestInterceptor: req => {
          req.headers['Authorization'] = `Bearer ${min.instance.webchatKey}`;
        }
      });

      const response = await client.apis.Conversations.Conversations_StartConversation();
      const conversationId = response.obj.conversationId;
      GBServer.globals.debugConversationId = conversationId;

      const steps = process.env.TEST_MESSAGE.split(';');
      const sleep = ms => {
        return new Promise(resolve => {
          setTimeout(resolve, ms);
        });
      };

      await CollectionUtil.asyncForEach(steps, async step => {
        client.apis.Conversations.Conversations_PostActivity({
          conversationId: conversationId,
          activity: {
            textFormat: 'plain',
            text: step,
            type: 'message',
            from: {
              id: 'test',
              name: 'test'
            }
          }
        });

        await sleep(3000);
      });
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

    GBDeployer.mountGBKBAssets(`${instance.botId}.gbkb`, instance.botId, `${instance.botId}.gbkb`);
  }

  public static isChatAPI(req: any, res: any) {
    if (!res) {
      return 'GeneralBots';
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
            const error = `WhatsApp API lost connection.`;
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
      // Checks request state by reading AntiCSRFAttackState from GB Admin infrastructure.

      const state = await min.adminService.getValue(instance.instanceId, 'AntiCSRFAttackState');
      if (req.query.state !== state) {
        const msg = 'WARNING: state field was not provided as anti-CSRF token';
        GBLog.error(msg);
        throw new Error(msg);
      }
      const authenticationContext = new AuthenticationContext.AuthenticationContext(
        urlJoin(min.instance.authenticatorAuthorityHostUrl, min.instance.authenticatorTenant)
      );
      const resource = 'https://graph.microsoft.com';

      // Calls MSFT to get token.

      authenticationContext.acquireTokenWithAuthorizationCode(
        req.query.code,
        urlJoin(instance.botEndpoint, min.instance.botId, '/token'),
        resource,
        instance.marketplaceId,
        instance.marketplacePassword,
        async (err, token) => {
          if (err) {
            const msg = `handleOAuthTokenRequests: Error acquiring token: ${err}`;
            GBLog.error(msg);
            res.send(msg);
          } else {
            // Saves token to the database.

            await this.adminService.setValue(instance.instanceId, 'accessToken', token['accessToken']);
            await this.adminService.setValue(instance.instanceId, 'refreshToken', token['refreshToken']);
            await this.adminService.setValue(instance.instanceId, 'expiresOn', token['expiresOn'].toString());
            await this.adminService.setValue(instance.instanceId, 'AntiCSRFAttackState', null);

            // Inform the home for default .gbui after finishing token retrival.

            res.redirect(min.instance.botEndpoint);
          }
        }
      );
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
      authorizationUrl = `${authorizationUrl}?response_type=code&client_id=${
        min.instance.marketplaceId
      }&redirect_uri=${urlJoin(min.instance.botEndpoint, min.instance.botId, 'token')}`;
      GBLog.info(`HandleOAuthRequests: ${authorizationUrl}.`);
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

    GBLog.info(`Client requested instance for: ${botId}.`);

    // Loads by the botId itself or by the activationCode field.

    let instance = await this.core.loadInstanceByBotId(botId);
    if (instance === null) {
      instance = await this.core.loadInstanceByActivationCode(botId);
    }

    if (instance !== null) {
      // Gets the webchat token, speech token and theme.

      const webchatTokenContainer = await this.getWebchatToken(instance);
      const speechToken = instance.speechKey != undefined ? await this.getSTSToken(instance) : null;
      let theme = instance.theme;

      // Sends all information to the .gbui web client.

      if (!theme) {
        theme = `default.gbtheme`;
      }

      res.send(
        JSON.stringify({
          instanceId: instance.instanceId,
          botId: botId,
          theme: theme,
          webchatToken: webchatTokenContainer.token,
          speechToken: speechToken,
          conversationId: webchatTokenContainer.conversationId,
          authenticatorTenant: instance.authenticatorTenant,
          authenticatorClientId: instance.marketplaceId,
          paramLogoImageUrl: this.core.getParam(instance, 'Logo Image Url', null),
          paramLogoImageAlt: this.core.getParam(instance, 'Logo Image Alt', null),
          paramLogoImageWidth: this.core.getParam(instance, 'Logo Image Width', null),
          paramLogoImageHeight: this.core.getParam(instance, 'Logo Image Height', null),
          paramLogoImageType: this.core.getParam(instance, 'Logo Image Type', null)
        })
      );
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

    try {
      const res = await fetch(instance.speechEndpoint, options);
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

    const adapter = new BotFrameworkAdapter({
      appId: instance.marketplaceId,
      appPassword: instance.marketplacePassword
    });
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
    min.packages = sysPackages;
    min.appPackages = appPackages;

    // NLP Manager.

    const manager = new NlpManager({ languages: ['pt'], forceNER: true });
    min['nerEngine'] = manager;

    if (GBServer.globals.minBoot === undefined) {
      GBServer.globals.minBoot = min;
    }

    if (min.instance.facebookWorkplaceVerifyToken) {
      min['fbAdapter'] = new FacebookAdapter({
        verify_token: min.instance.facebookWorkplaceVerifyToken,
        app_secret: min.instance.facebookWorkplaceAppSecret,
        access_token: min.instance.facebookWorkplaceAccessToken
      });
    }
    // https://github.com/GeneralBots/BotServer/issues/123
    // min.appPackages =  core.getPackagesByInstanceId(min.instance.instanceId);

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

    // If there is WhatsApp configuration specified, initialize
    // infrastructure objects.

    if (min.instance.whatsappServiceKey) {
      min.whatsAppDirectLine = new WhatsappDirectLine(
        min,
        min.botId,
        min.instance.whatsappBotKey,
        min.instance.whatsappServiceKey,
        min.instance.whatsappServiceNumber,
        min.instance.whatsappServiceUrl,
        group
      );

      await min.whatsAppDirectLine.setup(true);
    } else {
      const minBoot = GBServer.globals.minBoot as any;
      if (minBoot.whatsappServiceKey) {
        min.whatsAppDirectLine = new WhatsappDirectLine(
          min,
          min.botId,
          min.instance.whatsappBotKey,
          minBoot.instance.whatsappServiceKey,
          minBoot.instance.whatsappServiceNumber,
          minBoot.instance.whatsappServiceUrl,
          group
        );
        await min.whatsAppDirectLine.setup(false);
      }
    }

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
    instance: any,
    appPackages: any[]
  ) {
    // Uses standard or Facebook Adapter.

    let adapter = min.bot;
    if (req.body.object) {
      req['rawBody'] = JSON.stringify(req.body);
      adapter = min['fbAdapter'];
    }

    // Default activity processing and handler.

    await adapter['processActivity'](req, res, async context => {
      // Handle activity text issues.

      if (!context.activity.text) {
        context.activity.text = '';
      }
      context.activity.text = context.activity.text.replace(/\@General Bots Online /gi, '');

      // Get loaded user state

      const step = await min.dialogs.createContext(context);
      step.context.activity.locale = 'pt-BR';
      let firstTime = false;

      const member = context.activity.from;
      const sec = new SecService();
      const user = await sec.ensureUser(instance.instanceId, member.id, member.name, '', 'web', member.name, null);
      const userId = user.userId;
      const params = user.params ? JSON.parse(user.params) : {};

      try {
        const conversationReference = JSON.stringify(TurnContext.getConversationReference(context.activity));

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

          firstTime = true;

          // This same event is dispatched either to all participants
          // including the bot, that is filtered bellow.

          if (context.activity.from.id !== min.botId) {
            // Creates a new row in user table if it does not exists.

            // Stores conversation associated to the user to group each message.

            const analytics = new AnalyticsService();
            const conversation = await analytics.createConversation(user);
          }

          await sec.updateConversationReferenceById(userId, conversationReference);

          if (step.context.activity.channelId !== 'msteams') {
            const service = new KBService(min.core.sequelize);
            const data = await service.getFaqBySubjectArray(instance.instanceId, 'faq', undefined);
            await min.conversationalService.sendEvent(min, step, 'play', {
              playerType: 'bullet',
              data: data.slice(0, 10)
            });
          }
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
            const t = new SystemKeywords(null, null, null, null);
            const data = await t.getByHttp({
              pid: 0,
              url: file.contentUrl,
              headers,
              username: null,
              ps: null,
              qs: null
            });
            const folder = `work/${min.instance.botId}.gbai/cache`;
            const filename = `${GBAdminService.generateUuid()}.png`;

            Fs.writeFileSync(path.join(folder, filename), data);
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
              GBLog.info(`Auto start (teams) dialog is now being called: ${startDialog} for ${min.instance.botId}...`);
              await GBVMService.callVM(startDialog.toLowerCase(), min, step, user, this.deployer, false);
            }
          }
        }

        // Required for F0 handling of persisted conversations.

        GBLog.info(
          `Input> ${context.activity.text} (type: ${context.activity.type}, name: ${context.activity.name}, channelId: ${context.activity.channelId})`
        );

        // Answer to specific BOT Framework event conversationUpdate to auto start dialogs.
        // Skips if the bot is talking.
        const startDialog = min.core.getParam(min.instance, 'Start Dialog', null);

        if (context.activity.type === 'installationUpdate') {
          GBLog.info(`Bot installed on Teams.`);
        } else if (context.activity.type === 'conversationUpdate' && context.activity.membersAdded.length > 0) {
          // Check if a bot or a human participant is being added to the conversation.

          const member = context.activity.membersAdded[0];
          if (context.activity.membersAdded[0].id === context.activity.recipient.id) {
            GBLog.info(`Bot added to conversation, starting chat...`);

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
                min['conversationWelcomed'][step.context.activity.conversation.id] = true;

                GBLog.info(
                  `Auto start (web 1) dialog is now being called: ${startDialog} for ${min.instance.instanceId}...`
                );
                await GBVMService.callVM(startDialog.toLowerCase(), min, step, user, this.deployer, false);
              }
            }
          } else {
            GBLog.info(`Person added to conversation: ${member.name}`);

            if (GBMinService.userMobile(step)) {
              if (
                startDialog &&
                !min['conversationWelcomed'][step.context.activity.conversation.id] &&
                !step.context.activity['group']
              ) {
                await sec.setParam(userId, 'welcomed', 'true');
                min['conversationWelcomed'][step.context.activity.conversation.id] = true;
                GBLog.info(
                  `Auto start (whatsapp) dialog is now being called: ${startDialog} for ${min.instance.instanceId}...`
                );
                await GBVMService.callVM(startDialog.toLowerCase(), min, step, user, this.deployer, false);
              }
            }
          }
        } else if (context.activity.type === 'message') {
          // Processes messages activities.

          await this.processMessageActivity(context, min, step);
        } else if (context.activity.type === 'event') {
          // Processes events activities.

          await this.processEventActivity(min, user, context, step);
        }
      } catch (error) {
        const msg = `ERROR: ${error.message} ${error.stack ? error.stack : ''}`;
        GBLog.error(msg);

        await min.conversationalService.sendText(
          min,
          step,
          Messages[step.context.activity.locale].very_sorry_about_error
        );

        await step.beginDialog('/ask', { isReturning: true });
      }
    });
  }

  /**
   * Called to handle all event sent by .gbui clients.
   */
  private async processEventActivity(min, user, context, step: GBDialogStep) {
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
        GBLog.info(`Auto start (web 2) dialog is now being called: ${startDialog} for ${min.instance.instanceId}...`);
        await GBVMService.callVM(startDialog.toLowerCase(), min, step, user, this.deployer, false);
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
  private static async downloadAttachmentAndWrite(attachment) {
    const url = attachment.contentUrl;
    const localFolder = Path.join('work');
    const localFileName = Path.join(localFolder, `${this['min'].botId}.gbai`, 'uploads', attachment.name);

    let res;
    if (url.startsWith('data:')) {
      var regex = /^data:.+\/(.+);base64,(.*)$/;
      var matches = url.match(regex);
      var ext = matches[1];
      var data = matches[2];
      res = Buffer.from(data, 'base64');
    } else {
      // arraybuffer is necessary for images
      const options = {
        method: 'GET',
        encoding: 'binary'
      };
      res = await fetch(url, options);
      const buffer = arrayBufferToBuffer(await res.arrayBuffer());
      Fs.writeFileSync(localFileName, buffer);
    }

    // If no error was thrown while writing to disk,return the attachment's name
    // and localFilePath for the response back to the user.
    return {
      fileName: attachment.name,
      localPath: localFileName
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

  /**
   * Called to handle all text messages sent and received by the bot.
   */
  private async processMessageActivity(context, min: GBMinInstance, step: GBDialogStep) {
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

    let user = await sec.ensureUser(min.instance.instanceId, member.id, member.name, '', 'web', member.name, null);

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

        const conversationReference = JSON.stringify(TurnContext.getConversationReference(context.activity));
        await sec.updateConversationReferenceById(userId, conversationReference);
      }
    }

    // Prepare Promises to download each attachment and then execute each Promise.
    if (step.context.activity.attachments) {
      const promises = step.context.activity.attachments.map(
        GBMinService.downloadAttachmentAndWrite.bind({ min, user, params })
      );
      const successfulSaves = await Promise.all(promises);
      async function replyForReceivedAttachments(localAttachmentData) {
        if (localAttachmentData) {
          // Because the TurnContext was bound to this function,the bot can call
          // `TurnContext.sendActivity` via `this.sendActivity`;
          await this.sendActivity(`Upload OK.`);
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
        }

        const results = successfulSaves.reduce((accum: GBFile[], item) => {
          const result: GBFile = {
            data: Fs.readFileSync(successfulSaves[0]['localPath']),
            filename: successfulSaves[0]['fileName']
          };
          accum.push(result);
        }, []) as GBFile[];

        if (min.cbMap[userId] && min.cbMap[userId].promise == '!GBHEAR') {
          if (results.length > 1) {
            throw new Error('It is only possible to upload one file per message, right now.');
          }
          min.cbMap[userId].promise = results[0];
        }
      }
    }

    // Files in .gbdialog can be called directly by typing its name normalized into JS .

    const isVMCall = Object.keys(min.scriptMap).find(key => min.scriptMap[key] === context.activity.text) !== undefined;
    if (isVMCall) {
      await GBVMService.callVM(context.activity.text, min, step, user, this.deployer, false);
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
        await GBVMService.callVM(args, min, step, user, this.deployer, false);
      } else if (cmdOrDialogName === '/callsch') {
        await GBVMService.callVM(args, min, null, null, null, false);
      } else if (cmdOrDialogName === '/calldbg') {
        await GBVMService.callVM(args, min, step, user, this.deployer, true);
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

      let text = context.activity.text;
      const originalText = text;
      text = text.replace(/<([^>]+?)([^>]*?)>(.*?)<\/\1>/gi, '');

      // Saves special words (keep text) in tokens to prevent it from
      // spell checking and translation.

      const keepText: string = min.core.getParam(min.instance, 'Keep Text', '');
      let keepTextList = [];
      if (keepTextList) {
        keepTextList = keepTextList.concat(keepText.split(';'));
      }
      const replacements = [];
      await CollectionUtil.asyncForEach(min.appPackages, async (e: IGBPackage) => {
        const result = await e.onExchangeData(min, 'getKeepText', {});
        if (result) {
          keepTextList = keepTextList.concat(result);
        }
      });

      const getNormalizedRegExp = value => {
        var chars = [
          { letter: 'a', reg: '[aáàãäâ]' },
          { letter: 'e', reg: '[eéèëê]' },
          { letter: 'i', reg: '[iíìïî]' },
          { letter: 'o', reg: '[oóòõöô]' },
          { letter: 'u', reg: '[uúùüû]' },
          { letter: 'c', reg: '[cç]' }
        ];

        for (var i in chars) {
          value = value.replace(new RegExp(chars[i].letter, 'gi'), chars[i].reg);
        }
        return value;
      };

      let textProcessed = text;
      if (keepTextList) {
        keepTextList = keepTextList.filter(p => p.trim() !== '');
        let i = 0;
        await CollectionUtil.asyncForEach(keepTextList, item => {
          const it = GBConversationalService.removeDiacritics(item);
          const noAccentText = GBConversationalService.removeDiacritics(textProcessed);

          if (noAccentText.toLowerCase().indexOf(it.toLowerCase()) != -1) {
            const replacementToken = 'X' + GBAdminService.getNumberIdentifier().substr(0, 4);
            replacements[i] = { text: item, replacementToken: replacementToken };
            i++;
            textProcessed = textProcessed.replace(
              new RegExp(`\\b${getNormalizedRegExp(it.trim())}\\b`, 'gi'),
              `${replacementToken}`
            );
          }
        });
      }

      // Spells check the input text before translating,
      // keeping fixed tokens as specified in Config.

      text = await min.conversationalService.spellCheck(min, textProcessed);

      // Detects user typed language and updates their locale profile if applies.

      let locale = min.core.getParam<string>(
        min.instance,
        'Default User Language',
        GBConfigService.get('DEFAULT_USER_LANGUAGE')
      );
      const detectLanguage =
        min.core.getParam<boolean>(
          min.instance,
          'Language Detector',
          GBConfigService.getBoolean('LANGUAGE_DETECTOR')
        ) === 'true';
      locale = user.locale;
      if (text != '' && detectLanguage && !locale) {
        locale = await min.conversationalService.getLanguage(min, text);
        if (user.locale != locale) {
          user = await sec.updateUserLocale(user.userId, locale);
        }
      }

      // Checks for bad words on input text.

      const hasBadWord = wash.check(locale, context.activity.text);
      if (hasBadWord) {
        return await step.beginDialog('/pleaseNoBadWords');
      }

      // Translates text into content language, keeping
      // reserved tokens specified in Config.

      const contentLocale = min.core.getParam<string>(
        min.instance,
        'Default Content Language',
        GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
      );
      text = await min.conversationalService.translate(min, text, contentLocale);
      GBLog.verbose(`Translated text (processMessageActivity): ${text}.`);

      // Restores all token text back after spell checking and translation.

      if (keepTextList) {
        let i = 0;
        await CollectionUtil.asyncForEach(replacements, item => {
          i++;
          text = text.replace(new RegExp(`${item.replacementToken}`, 'gi'), item.text);
        });
      }
      step.context.activity['text'] = text;
      step.context.activity['originalText'] = originalText;

      GBLog.info(`Text>: ${text}.`);

      if (user.agentMode === 'self') {
        const manualUser = await sec.getUserFromAgentSystemId(user.userSystemId);

        GBLog.info(`HUMAN AGENT (${user.userId}) TO USER ${manualUser.userSystemId}: ${text}`);

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
            locale,
            step.context.activity.conversation.id
          );
        }
      } else {
        if (min.cbMap[userId] && min.cbMap[userId].promise == '!GBHEAR') {
          min.cbMap[userId].promise = text;
        }

        // If there is a dialog in course, continue to the next step.
        else if (step.activeDialog !== undefined) {
          await step.continueDialog();
        } else {
          const startDialog = user.hearOnDialog
            ? user.hearOnDialog
            : min.core.getParam(min.instance, 'Start Dialog', null);

          if (text !== startDialog) {
            let nextDialog = null;
            let data = {
              query: text,
              step: step,
              notTranslatedQuery: originalText,
              message: message ? message['dataValues'] : null,
              user: user ? user.dataValues : null
            };
            await CollectionUtil.asyncForEach(min.appPackages, async (e: IGBPackage) => {
              if (!nextDialog) {
                nextDialog = await e.onExchangeData(min, 'handleAnswer', data);
              }
            });
            data.step = null;
            GBLog.info(`/answer being called from processMessageActivity (nextDialog=${nextDialog}).`);
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
}
