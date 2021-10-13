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
const { DialogSet, TextPrompt } = require('botbuilder-dialogs');
const express = require('express');
const request = require('request-promise-native');
const removeRoute = require('express-remove-route');
const AuthenticationContext = require('adal-node').AuthenticationContext;
const wash = require('washyourmouthoutwithsoap');
const { FacebookAdapter } = require('botbuilder-adapter-facebook');
import {
  AutoSaveStateMiddleware,
  BotFrameworkAdapter,
  ConversationState,
  MemoryStorage,
  TurnContext,
  UserState
} from 'botbuilder';
import { ConfirmPrompt, OAuthPrompt, WaterfallDialog } from 'botbuilder-dialogs';
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
import { GBServer } from '../../../src/app';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService';
import { GuaribasConversationMessage } from '../../analytics.gblib/models';
import { AnalyticsService } from '../../analytics.gblib/services/AnalyticsService';
import { GBVMService } from '../../basic.gblib/services/GBVMService';
import { AskDialogArgs } from '../../kb.gbapp/dialogs/AskDialog';
import { KBService } from '../../kb.gbapp/services/KBService';
import { SecService } from '../../security.gbapp/services/SecService';
import { WhatsappDirectLine } from '../../whatsapp.gblib/services/WhatsappDirectLine';
import { Messages } from '../strings';
import { GBConfigService } from './GBConfigService';
import { GBConversationalService } from './GBConversationalService';
import { GBDeployer } from './GBDeployer';
import urlJoin = require('url-join');
import fs = require('fs');
import { GoogleChatDirectLine } from '../../google-chat.gblib/services/GoogleChatDirectLine';
import { ScheduleServices } from '../../basic.gblib/services/ScheduleServices';

/**
 * Minimal service layer for a bot and encapsulation of BOT Framework calls.
 */
export class GBMinService {

  /**
   * Default General Bots User Interface package.
   */
  private static uiPackage = 'default.gbui';

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
      const url = GBServer.globals.wwwroot
        ? GBServer.globals.wwwroot
        : urlJoin(GBDeployer.deployFolder, GBMinService.uiPackage, 'build');

      GBServer.globals.server.use('/', express.static(url));
    }

    // Servers the bot information object via HTTP so clients can get
    // instance information stored on server.

    if (process.env.DISABLE_WEB !== 'true') {
      GBServer.globals.server.get('/instances/:botId', this.handleGetInstanceForClient.bind(this));
    }

    // Servers the WhatsApp callback.

    GBServer.globals.server.post('/webhooks/whatsapp', this.WhatsAppCallback.bind(this));

    // Call mountBot event to all bots.

    await CollectionUtil.asyncForEach(instances, async instance => {
      try {
        await this.mountBot(instance);
      } catch (error) {
        GBLog.error(`Error mounting bot ${instance.botId}: ${error.message}\n${error.stack}`);
      }
    });

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
    GBServer.globals.minInstances.push(min);

    await this.deployer.deployPackage(min, 'packages/default.gbtheme');

    // Install per bot deployed packages.

    let packagePath = `work/${min.botId}.gbai/${min.botId}.gbdialog`;
    if (fs.existsSync(packagePath)) {
      await this.deployer.deployPackage(min, packagePath);
    }
    packagePath = `work/${min.botId}.gbai/${min.botId}.gbapp`;
    if (fs.existsSync(packagePath)) {
      await this.deployer.deployPackage(min, packagePath);
    }
    packagePath = `work/${min.botId}.gbai/${min.botId}.gbtheme`;
    if (fs.existsSync(packagePath)) {
      await this.deployer.deployPackage(min, packagePath);
    }
    packagePath = `work/${min.botId}.gbai/${min.botId}.gblib`;
    if (fs.existsSync(packagePath)) {
      await this.deployer.deployPackage(min, packagePath);
    }

    const service = new ScheduleServices();
    await service.loadSchedules(min);

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
    GBLog.info(`GeneralBots(${instance.engineName}) listening on: ${url}.`);

    // Serves individual URL for each bot user interface.

    if (process.env.DISABLE_WEB !== 'true') {
      const uiUrl = `/${instance.botId}`;
      const uiUrlAlt = `/${instance.activationCode}`;
      GBServer.globals.server.use(
        uiUrl,
        express.static(urlJoin(GBDeployer.deployFolder, GBMinService.uiPackage, 'build'))
      );
      GBServer.globals.server.use(
        uiUrlAlt,
        express.static(urlJoin(GBDeployer.deployFolder, GBMinService.uiPackage, 'build'))
      );
      const domain = min.core.getParam(min.instance, 'Domain', null);
      if (domain) {
        GBServer.globals.server.use(
          domain,
          express.static(urlJoin(GBDeployer.deployFolder, GBMinService.uiPackage, 'build'))
        );
        GBLog.info(`Bot UI ${GBMinService.uiPackage} accessible at custom domain: ${domain}.`);
      }
      GBLog.info(`Bot UI ${GBMinService.uiPackage} accessible at: ${uiUrl} and ${uiUrlAlt}.`);
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
  }

  private async WhatsAppCallback(req, res) {
    try {

      // Detects if the message is echo fro itself.

      const id = req.body.messages[0].chatId.split('@')[0];
      const senderName = req.body.messages[0].senderName;
      const text = req.body.messages[0].body;
      if (req.body.messages[0].fromMe) {
        res.end();

        return; // Exit here.
      }

      // Detects if the welcome message is enabled.

      let activeMin;
      if (process.env.WHATSAPP_WELCOME_DISABLED !== 'true') {

        // Tries to find if user wants to switch bots.

        let toSwitchMin = GBServer.globals.minInstances.filter(
          p => p.instance.botId.toLowerCase() === text.toLowerCase()
        )[0];
        if (!toSwitchMin) {
          toSwitchMin = GBServer.globals.minInstances.filter(p =>
            p.instance.activationCode ? p.instance.activationCode.toLowerCase() === text.toLowerCase() : false
          )[0];
        }

        // Find active bot instance.

        activeMin = toSwitchMin ? toSwitchMin : GBServer.globals.minBoot;

        // If it is the first time for the user, tries to auto-execute
        // start dialog if any is specified in Config.xlsx.

        const sec = new SecService();
        let user = await sec.getUserFromSystemId(id);
        if (user === null || user.hearOnDialog) {
          user = await sec.ensureUser(activeMin.instance.instanceId, id, senderName, '', 'whatsapp', senderName, null);

          const startDialog = user.hearOnDialog ?
            user.hearOnDialog :
            activeMin.core.getParam(activeMin.instance, 'Start Dialog', null);

          if (startDialog) {
            GBLog.info(`Calling /start to Auto start ${startDialog} for ${activeMin.instance.instanceId}...`);
            req.body.messages[0].body = `/start`;

            // Resets HEAR ON DIALOG value to none and passes
            // current dialog to the direct line.

            await sec.updateUserHearOnDialog(user.userId, null);
            await (activeMin as any).whatsAppDirectLine.received(req, res);
          } else {
            await (activeMin as any).whatsAppDirectLine.sendToDevice(
              id,
              `Olá! Seja bem-vinda(o)!\nMe chamo ${activeMin.instance.title}. Como posso ajudar? Pode me falar que eu te ouço, me manda um aúdio.`
            );
            res.end();
          }
        } else {

          // User wants to switch bots.

          if (toSwitchMin !== undefined) {

            // So gets the new bot instance information and prepares to
            // auto start dialog if any is specified.

            const instance = await this.core.loadInstanceByBotId(activeMin.botId);
            await sec.updateUserInstance(id, instance.instanceId);
            await (activeMin as any).whatsAppDirectLine.resetConversationId(id);
            const startDialog = activeMin.core.getParam(activeMin.instance, 'Start Dialog', null);


            if (startDialog) {
              GBLog.info(`Calling /start for Auto start : ${startDialog} for ${activeMin.instance.botId}...`);
              req.body.messages[0].body = `/start`;
              await (activeMin as any).whatsAppDirectLine.received(req, res);
            } else {
              await (activeMin as any).whatsAppDirectLine.sendToDevice(
                id,
                `Agora falando com ${activeMin.instance.title}...`
              );
            }
            res.end();
          } else {
            activeMin = GBServer.globals.minInstances.filter(p => p.instance.instanceId === user.instanceId)[0];
            if (activeMin === undefined) {
              activeMin = GBServer.globals.minBoot;
              await (activeMin as any).whatsAppDirectLine.sendToDevice(
                id,
                `O outro Bot que você estava falando(${user.instanceId}), não está mais disponível. Agora você está falando comigo, ${activeMin.instance.title}...`
              );
            }
            await (activeMin as any).whatsAppDirectLine.received(req, res);
          }
        }
      } else {

        // Just pass the message to the receiver.

        await (GBServer.globals.minBoot as any).whatsAppDirectLine.received(req, res);
      }
    } catch (error) {
      GBLog.error(`Error on Whatsapp callback: ${error.data ? error.data : error}`);
    }
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
      const authenticationContext = new AuthenticationContext(
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

            await this.adminService.setValue(instance.instanceId, 'accessToken', token.accessToken);
            await this.adminService.setValue(instance.instanceId, 'refreshToken', token.refreshToken);
            await this.adminService.setValue(instance.instanceId, 'expiresOn', token.expiresOn.toString());
            await this.adminService.setValue(instance.instanceId, 'AntiCSRFAttackState', undefined);

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
      authorizationUrl = `${authorizationUrl}?response_type=code&client_id=${min.instance.marketplaceId
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
    const options = {
      url: 'https://directline.botframework.com/v3/directline/tokens/generate',
      method: 'POST',
      headers: {
        Authorization: `Bearer ${instance.webchatKey}`
      }
    };

    try {
      const json = await request(options);

      return JSON.parse(json);
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
      url: instance.speechEndpoint,
      method: 'POST',
      headers: {
        'Ocp-Apim-Subscription-Key': instance.speechKey
      }
    };

    try {
      return await request(options);
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

    const adapter = new BotFrameworkAdapter(
      { appId: instance.marketplaceId, appPassword: instance.marketplacePassword });
    const storage = new MemoryStorage();
    const conversationState = new ConversationState(storage);
    const userState = new UserState(storage);
    adapter.use(new AutoSaveStateMiddleware(conversationState, userState));
    MicrosoftAppCredentials.trustServiceUrl('https://directline.botframework.com',
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
    min["scheduleMap"] = {};
    min["conversationWelcomed"] = {};
    min.packages = sysPackages;
    min.appPackages = appPackages;

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
    // TODO: min.appPackages =  core.getPackagesByInstanceId(min.instance.instanceId);

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
    // If there is WhatsApp configuration specified, initialize
    // infrastructure objects.

    if (min.instance.whatsappServiceUrl) {
      min.whatsAppDirectLine = new WhatsappDirectLine(
        min,
        min.botId,
        min.instance.whatsappBotKey,
        min.instance.whatsappServiceKey,
        min.instance.whatsappServiceNumber,
        min.instance.whatsappServiceUrl
      );
      await min.whatsAppDirectLine.setup(true);
    } else {
      const minBoot = GBServer.globals.minBoot as any;
      if (minBoot.instance.whatsappServiceUrl) {
        min.whatsAppDirectLine = new WhatsappDirectLine(
          min,
          min.botId,
          min.instance.whatsappBotKey,
          minBoot.instance.whatsappServiceKey,
          minBoot.instance.whatsappServiceNumber,
          minBoot.instance.whatsappServiceUrl
        );
        await min.whatsAppDirectLine.setup(false);
      }
    }

    // Setups default BOT Framework dialogs.

    min.userProfile = conversationState.createProperty('userProfile');
    const dialogState = conversationState.createProperty('dialogState');

    min.dialogs = new DialogSet(dialogState);
    min.dialogs.add(new TextPrompt('textPrompt'));
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

  // TODO: Unify in util.
  public static userMobile(step) {
    let mobile = WhatsappDirectLine.mobiles[step.context.activity.conversation.id]
    return mobile;

    if (isNaN(step.context.activity['mobile'])) {
      if (step.context.activity.from && !isNaN(step.context.activity.from.id)) {
        return step.context.activity.from.id;
      }
      return null;
    } else {
      return step.context.activity['mobile'];
    }
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

    let adapter = min.bot;

    if (req.body.object) {
      req['rawBody'] = JSON.stringify(req.body);
      adapter = min['fbAdapter'];
    }

    // Default activity processing and handler.

    await adapter['processActivity'](req, res, async context => {

      // Get loaded user state

      const step = await min.dialogs.createContext(context);
      step.context.activity.locale = 'pt-BR';
      let firstTime = false;


      try {
        const user = await min.userProfile.get(context, {});

        // First time processing.

        const sec = new SecService();
        if (!user.loaded) {

          await min.conversationalService.sendEvent(min, step, 'loadInstance', {});

          user.loaded = true;
          user.subjects = [];
          user.cb = undefined;
          user.welcomed = false;
          user.basicOptions = { maxLines: 100, translatorOn: true, wholeWord: true };

          firstTime = true;

          const service = new KBService(min.core.sequelize);
          const data = await service.getFaqBySubjectArray(instance.instanceId, 'faq', undefined);
          await min.conversationalService.sendEvent(min, step, 'play', {
            playerType: 'bullet',
            data: data.slice(0, 10)
          });

          // This same event is dispatched either to all participants
          // including the bot, that is filtered bellow.

          if (context.activity.from.id !== min.botId) {

            // Creates a new row in user table if it does not exists.

            const member = context.activity.from;
            const persistedUser = await sec.ensureUser(
              instance.instanceId,
              member.id,
              member.name,
              '',
              'web',
              member.name,
              null
            );


            // Stores conversation associated to the user to group each message.

            const analytics = new AnalyticsService();
            user.systemUser = persistedUser;
            user.conversation = await analytics.createConversation(persistedUser);

          }

          // Saves session user (persisted GuaribasUser is inside).

          await min.userProfile.set(step.context, user);
        }

        user.systemUser = await sec.getUserFromSystemId(user.systemUser.userSystemId);
        await min.userProfile.set(step.context, user);

        // Required for MSTEAMS handling of persisted conversations.

        if (step.context.activity.channelId === 'msteams') {
          const conversationReference = JSON.stringify(
            TurnContext.getConversationReference(context.activity)
          );
          await sec.updateConversationReferenceById(user.systemUser.userId, conversationReference);

          if (!user.welcomed) {
            const startDialog = min.core.getParam(min.instance, 'Start Dialog', null);
            if (startDialog && !user.welcomed) {
              user.welcomed = true;
              GBLog.info(`Auto start (teams) dialog is now being called: ${startDialog} for ${min.instance.botId}...`);
              await GBVMService.callVM(startDialog.toLowerCase(), min, step, this.deployer);


            }
          }
        }

        // Required for F0 handling of persisted conversations.

        GBLog.info(`User>: text:${context.activity.text} (type: ${context.activity.type}, name: ${context.activity.name}, channelId: ${context.activity.channelId}, value: ${context.activity.value})`);

        // Answer to specific BOT Framework event conversationUpdate to auto start dialogs.
        // Skips if the bot is talking.
        const startDialog = min.core.getParam(min.instance, 'Start Dialog', null);

        if (context.activity.type === 'installationUpdate') {
          GBLog.info(`Bot installed on Teams.`);
        } else if (context.activity.type === 'conversationUpdate' &&
          context.activity.membersAdded.length > 0) {

          // Check if a bot or a human participant is being added to the conversation.

          const member = context.activity.membersAdded[0];
          if (context.activity.membersAdded[0].id === context.activity.recipient.id) {
            GBLog.info(`Bot added to conversation, starting chat...`);

            // Calls onNewSession event on each .gbapp package.

            await CollectionUtil.asyncForEach(appPackages, async e => {
              await e.onNewSession(min, step);
            });

            // Auto starts dialogs if any is specified.

            if (!startDialog && !user.welcomed) {

              // Otherwise, calls / (root) to default welcome users.

              await step.beginDialog('/');
            }
            else {
              if (!min["conversationWelcomed"][step.context.activity.conversation.id]) {

                min["conversationWelcomed"][step.context.activity.conversation.id] = true;

                GBLog.info(`Auto start (web) dialog is now being called: ${startDialog} for ${min.instance.instanceId}...`);
                await GBVMService.callVM(startDialog.toLowerCase(), min, step, this.deployer);
              }
            }

          } else {
            GBLog.info(`Person added to conversation: ${member.name}`);

            if (GBMinService.userMobile(step)) {
              if (startDialog && !min["conversationWelcomed"][step.context.activity.conversation.id]) {
                user.welcomed = true;
                await min.userProfile.set(step.context, user);
                GBLog.info(`Auto start (whatsapp) dialog is now being called: ${startDialog} for ${min.instance.instanceId}...`);
                await GBVMService.callVM(startDialog.toLowerCase(), min, step, this.deployer);
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

        // Saves conversation state for later use.

        await conversationState.saveChanges(context, true);

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
      if (startDialog && !min["conversationWelcomed"][step.context.activity.conversation.id]) {
        user.welcomed = true;
        GBLog.info(`Auto start (web) dialog is now being called: ${startDialog} for ${min.instance.instanceId}...`);
        await GBVMService.callVM(startDialog.toLowerCase(), min, step, this.deployer);
      }
    } else if (context.activity.name === 'updateToken') {
      const token = context.activity.data;
      await step.beginDialog('/adminUpdateToken', { token: token });
    } else {
      await step.continueDialog();
    }
  }

  /**
   * Called to handle all text messages sent and received by the bot.
   */
  private async processMessageActivity(context, min: GBMinInstance, step: GBDialogStep) {

    const sec = new SecService();

    // Removes <at>Bot Id</at> from MS Teams.

    context.activity.text = context.activity.text.replace(/\<at\>.*\<\/at\>\s/gi, '');

    let data = { query: context.activity.text };
    await CollectionUtil.asyncForEach(min.appPackages, async (e: IGBPackage) => {
      await e.onExchangeData(min, 'handleRawInput', data);
      // TODO: Handle priority over .gbapp, today most common case is just one item per server.
    });
    context.activity.text = data.query;

    // Additional clean up.

    context.activity.text = context.activity.text.trim();

    const user = await min.userProfile.get(context, {});
    let message: GuaribasConversationMessage;
    if (process.env.PRIVACY_STORE_MESSAGES === 'true') {

      // Adds message to the analytics layer.

      const analytics = new AnalyticsService();
      if (user) {

        if (!user.conversation) {
          user.conversation = await analytics.createConversation(user.systemUser);
        }

        message = await analytics.createMessage(
          min.instance.instanceId,
          user.conversation,
          user.systemUser.userId,
          context.activity.text
        );
      }
    }

    // Checks for global exit kewywords cancelling any active dialogs.

    const globalQuit = (locale, utterance) => {
      return utterance.match(Messages.global_quit);
    };

    // Files in .gbdialog can be called directly by typing its name normalized into JS .

    const isVMCall = Object.keys(min.scriptMap).find(key => min.scriptMap[key] === context.activity.text) !== undefined;
    if (isVMCall) {
      await GBVMService.callVM(context.activity.text, min, step, this.deployer);
    } else if (context.activity.text.charAt(0) === '/') {

      const text = context.activity.text;
      const parts = text.split(' ');
      const cmdOrDialogName = parts[0];
      parts.splice(0, 1);
      const args = parts.join(' ');
      if (cmdOrDialogName === '/start') {
        // TODO: Args to BASIC.
      } else if (cmdOrDialogName === '/call') {
        await GBVMService.callVM(args, min, step, this.deployer);
      } else {
        await step.beginDialog(cmdOrDialogName, { args: args });
      }
    } else if (globalQuit(step.context.activity.locale, context.activity.text)) {
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
      await min.conversationalService.sendText(min, step,
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

      const getNormalizedRegExp = (value) => {
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
        };
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
            textProcessed = textProcessed.replace(new RegExp(`\\b${getNormalizedRegExp(it.trim())}\\b`, 'gi'), `${replacementToken}`);
          }
        });
      }

      // Spells check the input text before translating,
      // keeping fixed tokens as specified in Config.

      text = await min.conversationalService.spellCheck(min, textProcessed);

      // Detects user typed language and updates their locale profile if applies.

      let locale = min.core.getParam<string>(min.instance, 'Default User Language',
        GBConfigService.get('DEFAULT_USER_LANGUAGE')
      );
      const detectLanguage = min.core.getParam<boolean>(min.instance, 'Language Detector',
        GBConfigService.getBoolean('LANGUAGE_DETECTOR')
      ) === 'true';
      const systemUser = user.systemUser;
      locale = systemUser.locale;
      if (detectLanguage || !locale) {
        locale = await min.conversationalService.getLanguage(min, text);
        if (systemUser.locale != locale) {

          user.systemUser = await sec.updateUserLocale(systemUser.userId, locale);
          await min.userProfile.set(step.context, user);
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
      GBLog.info(`Translated text (processMessageActivity): ${text}.`);

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

      GBLog.info(`Final text ready for NLP/Search/.gbapp: ${text}.`);

      if (user.systemUser.agentMode === 'self') {
        const manualUser = await sec.getUserFromAgentSystemId(user.systemUser.userSystemId);

        GBLog.info(`HUMAN AGENT (${user.systemUser.userSystemId}) TO USER ${manualUser.userSystemId}: ${text}`);

        const cmd = 'SEND FILE ';
        if (text.startsWith(cmd)) {
          const filename = text.substr(cmd.length);
          const message = await min.kbService.getAnswerTextByMediaName(min.instance.instanceId, filename);

          if (message === null) {
            GBLog.error(`File ${filename} not found in any .gbkb published. Check the name or publish again the associated .gbkb.`);
          } else {
            await min.conversationalService.sendMarkdownToMobile(min, null, manualUser.userSystemId, message);
          }
        }
        else {
          await min.whatsAppDirectLine.sendToDeviceEx(manualUser.userSystemId, `${manualUser.agentSystemId}: ${text}`, locale);
        }
      }
      else {

        // If there is a dialog in course, continue to the next step.

        if (step.activeDialog !== undefined) {
          await step.continueDialog();
        } else {

          const startDialog = user.hearOnDialog ?
            user.hearOnDialog :
            min.core.getParam(min.instance, 'Start Dialog', null);

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
