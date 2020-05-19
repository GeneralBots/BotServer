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

import urlJoin = require('url-join');
const { DialogSet, TextPrompt } = require('botbuilder-dialogs');
const express = require('express');
const request = require('request-promise-native');
const removeRoute = require('express-remove-route');
const AuthenticationContext = require('adal-node').AuthenticationContext;
const wash = require('washyourmouthoutwithsoap');
import { AutoSaveStateMiddleware, BotFrameworkAdapter, ConversationState, MemoryStorage, UserState } from 'botbuilder';
import { CollectionUtil, AzureText } from 'pragmatismo-io-framework';
import { ConfirmPrompt, WaterfallDialog } from 'botbuilder-dialogs';
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

import { MicrosoftAppCredentials } from 'botframework-connector';
import { GBServer } from '../../../src/app';
import { AskDialogArgs } from '../../kb.gbapp/dialogs/AskDialog';
import { KBService } from '../../kb.gbapp/services/KBService';
import { Messages } from '../strings';
import { GBConfigService } from './GBConfigService';
import { GBDeployer } from './GBDeployer';
import { SecService } from '../../security.gblib/services/SecService';
import { AnalyticsService } from '../../analytics.gblib/services/AnalyticsService';
import { WhatsappDirectLine } from '../../whatsapp.gblib/services/WhatsappDirectLine';
import fs = require('fs');
import { GBConversationalService } from './GBConversationalService';

/**
 * Minimal service layer for a bot.
 */
export class GBMinService {
  public core: IGBCoreService;
  public conversationalService: IGBConversationalService;
  public adminService: IGBAdminService;
  public deployer: GBDeployer;

  private static uiPackage = 'default.gbui';

  public corePackage = 'core.gbai';

  /**
   * Static initialization of minimal instance.
   *
   * @param core Basic database services to identify instance, for example.
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
   *
   * Constructs a new minimal instance for each bot.
   *
   * @param server        An HTTP server.
   * @param appPackages   List of loaded .gbapp associated with this instance.
   *
   * @return Loaded minimal bot instance.
   *
   */

  public async buildMin(
    instances: IGBInstance[],
  ) {
    // Serves default UI on root address '/' if web enabled.
    if (process.env.DISABLE_WEB !== 'true') {
      let url = GBServer.globals.wwwroot ?
        GBServer.globals.wwwroot :
        urlJoin(GBDeployer.deployFolder, GBMinService.uiPackage, 'build');

      GBServer.globals.server.use('/', express.static(url));
    }
    // Serves the bot information object via HTTP so clients can get
    // instance information stored on server.
    if (process.env.DISABLE_WEB !== 'true') {
      GBServer.globals.server.get('/instances/:botId', (req, res) => {
        (async () => {
          await this.handleGetInstanceForClient(req, res);
        })();
      });
    }
    const url = '/webhooks/whatsapp';
    GBServer.globals.server.post(url, async (req, res) => {
      try {

        const id = req.body.messages[0].chatId.split('@')[0];
        const senderName = req.body.messages[0].senderName;
        const text = req.body.messages[0].body;
        if (req.body.messages[0].fromMe) {
          res.end();
          return; // Exit here.
        }
        let activeMin;
        if (process.env.WHATSAPP_WELCOME_DISABLED !== "true") {

          const toSwitchMin = GBServer.globals.minInstances.filter(p => p.instance.activationCode === text)[0];
          activeMin = toSwitchMin ? toSwitchMin : GBServer.globals.minBoot;

          let sec = new SecService();
          
          let user = await sec.getUserFromSystemId(id);

          if (user === null) {
            user = await sec.ensureUser(activeMin.instance.instanceId, id, senderName, "", "whatsapp", senderName);
            await (activeMin as any).whatsAppDirectLine.sendToDevice(id, `Olá! Seja bem-vinda(o)!\nMe chamo ${activeMin.instance.title}. Como posso ajudar? Pode me falar que eu te ouço, me manda um aúdio.`);
            res.end();
          } else {
            // User wants to switch bots.
            if (toSwitchMin !== undefined) {
              const botId = text;
              const instance = await this.core.loadInstanceByBotId(botId);
              await sec.updateUserInstance(id, instance.instanceId);

              await (activeMin as any).whatsAppDirectLine.resetConversationId(id);
              await (activeMin as any).whatsAppDirectLine.sendToDevice(id, `Agora falando com ${activeMin.instance.title}...`);
              res.end();
            }
            else {
              activeMin = GBServer.globals.minInstances.filter(p => p.instance.instanceId === user.instanceId)[0];;
              if (activeMin === undefined) {
                activeMin = GBServer.globals.minBoot;
                await (activeMin as any).whatsAppDirectLine.sendToDevice(id, `O outro Bot que você estava falando(${user.instanceId}), não está mais disponível. Agora você está falando comigo, ${activeMin.instance.title}...`);
              }
              await (activeMin as any).whatsAppDirectLine.received(req, res);
            }
          }
        }
        else {
          await (GBServer.globals.minBoot as any).whatsAppDirectLine.received(req, res);
        }
      } catch (error) {
        GBLog.error(`Error on Whatsapp callback: ${error.message}`);
      }

    });

    await CollectionUtil.asyncForEach(instances, async instance => {
      try {
        await this.mountBot(instance);
      } catch (error) {
        GBLog.error(`Error mounting bot ${instance.botId}: ${error.message}`);
      }
    });
  }

  public async unmountBot(botId: string) {
    const url = `/api/messages/${botId}`;
    removeRoute(GBServer.globals.server, url);

    const uiUrl = `/${botId}`;
    removeRoute(GBServer.globals.server, uiUrl);

    GBServer.globals.minInstances = GBServer.globals.minInstances.filter(p => p.instance.botId !== botId);
  }

  public async mountBot(instance: IGBInstance) {

    // Build bot adapter.
    const { min, adapter, conversationState } = await this.buildBotAdapter(instance, GBServer.globals.sysPackages);
    GBServer.globals.minInstances.push(min);

    await this.deployer.deployPackage(min, 'packages/default.gbtheme');

    // Install per bot deployed packages.

    let packagePath = `work/${min.botId}.gbdialog`;
    if (fs.existsSync(packagePath)) {
      await this.deployer.deployPackage(min, packagePath);
    }
    packagePath = `work/${min.botId}.gbapp`;
    if (fs.existsSync(packagePath)) {
      await this.deployer.deployPackage(min, packagePath);
    }
    packagePath = `work/${min.botId}.gbtheme`;
    if (fs.existsSync(packagePath)) {
      await this.deployer.deployPackage(min, packagePath);
    }
    packagePath = `work/${min.botId}.gblib`;
    if (fs.existsSync(packagePath)) {
      await this.deployer.deployPackage(min, packagePath);
    }

    // Call the loadBot context.activity for all packages.
    await this.invokeLoadBot(GBServer.globals.appPackages, GBServer.globals.sysPackages, min);

    // Serves individual URL for each bot conversational interface...
    const url = `/api/messages/${instance.botId}`;
    GBServer.globals.server.post(url, async (req, res) => {
      await this.receiver(adapter, req, res, conversationState, min, instance, GBServer.globals.appPackages);
    });
    GBLog.info(`GeneralBots(${instance.engineName}) listening on: ${url}.`);

    // Serves individual URL for each bot user interface.
    if (process.env.DISABLE_WEB !== 'true') {
      const uiUrl = `/${instance.botId}`;
      const uiUrlAlt = `/${instance.activationCode}`;
      GBServer.globals.server.use(uiUrl, express.static(urlJoin(GBDeployer.deployFolder, GBMinService.uiPackage, 'build')));
      GBServer.globals.server.use(uiUrlAlt, express.static(urlJoin(GBDeployer.deployFolder, GBMinService.uiPackage, 'build')));

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

    this.createCheckHealthAddress(GBServer.globals.server, min, min.instance);
  }

  private createCheckHealthAddress(server: any, min: GBMinInstance, instance: IGBInstance) {
    server.get(`/${min.instance.botId}/check`, async (req, res) => {
      try {
        if (min.whatsAppDirectLine != undefined && instance.whatsappServiceKey !== null) {
          if (!await min.whatsAppDirectLine.check(min)) {
            const error = `WhatsApp API lost connection.`;
            GBLog.error(error);
            res.status(500).send(error);

            return;
          }
        }
        res.status(200).send(`General Bot ${min.botId} is healthly.`);
      } catch (error) {
        GBLog.error(error);
        res.status(500).send(error.toString());
      }
    });
  }


  private handleOAuthTokenRequests(server: any, min: GBMinInstance, instance: IGBInstance) {
    server.get(`/${min.instance.botId}/token`, async (req, res) => {
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
      authenticationContext.acquireTokenWithAuthorizationCode(
        req.query.code,
        urlJoin(instance.botEndpoint, min.instance.botId, '/token'),
        resource,
        instance.marketplaceId,
        instance.marketplacePassword,
        async (err, token) => {
          if (err) {
            const msg = `Error acquiring token: ${err}`;
            GBLog.error(msg);
            res.send(msg);
          } else {
            this.adminService.setValue(instance.instanceId, 'refreshToken', token.refreshToken);
            this.adminService.setValue(instance.instanceId, 'accessToken', token.accessToken);
            this.adminService.setValue(instance.instanceId, 'expiresOn', token.expiresOn.toString());
            this.adminService.setValue(instance.instanceId, 'AntiCSRFAttackState', undefined);
            res.redirect(min.instance.botEndpoint);
          }
        }
      );
    });
  }

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
      res.redirect(authorizationUrl);
    });
  }

  /**
   * Returns the instance object to clients requesting bot info.
   */
  private async handleGetInstanceForClient(req: any, res: any) {
    let botId = req.params.botId;
    if (botId === '[default]' || botId === undefined) {
      botId = GBConfigService.get('BOT_ID');
    }
    const instance = await this.core.loadInstanceByBotId(botId);
    if (instance !== null) {
      const webchatTokenContainer = await this.getWebchatToken(instance);
      const speechToken = instance.speechKey != null ? await this.getSTSToken(instance) : null;
      let theme = instance.theme;
      if (theme === undefined) {
        theme = 'default.gbtheme';
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
          authenticatorClientId: instance.marketplaceId
        })
      );
    } else {
      const error = `Instance not found: ${botId}.`;
      res.sendStatus(error);
      GBLog.error(error);
    }
  }

  /**
   * Get Webchat key from Bot Service.
   *
   * @param instance The Bot instance.
   *
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

      return Promise.resolve(JSON.parse(json));
    } catch (error) {
      const msg = `[botId:${
        instance.botId
        }] Error calling Direct Line client, verify Bot endpoint on the cloud. Error is: ${error}.`;

      return Promise.reject(new Error(msg));
    }
  }

  /**
   * Gets a Speech to Text / Text to Speech token from the provider.
   *
   * @param instance The general bot instance.
   *
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

  private async buildBotAdapter(instance: any, sysPackages: IGBPackage[]) {
    const adapter = new BotFrameworkAdapter({
      appId: instance.marketplaceId,
      appPassword: instance.marketplacePassword
    });
    const storage = new MemoryStorage();

    const conversationState = new ConversationState(storage);
    const userState = new UserState(storage);
    adapter.use(new AutoSaveStateMiddleware(conversationState, userState));

    MicrosoftAppCredentials.trustServiceUrl('https://directline.botframework.com',
      new Date(new Date().setFullYear(new Date().getFullYear() + 10)));

    // The minimal bot is built here.

    const min = new GBMinInstance();

    if (GBServer.globals.minBoot === undefined) {
      GBServer.globals.minBoot = min;
    }

    min.botId = instance.botId;
    min.bot = adapter;
    min.userState = userState;
    min.core = this.core;
    min.conversationalService = this.conversationalService;
    min.adminService = this.adminService;
    min.deployService = this.deployer;
    min.kbService = new KBService(this.core.sequelize);
    min.instance = await this.core.loadInstanceByBotId(min.botId);
    min.cbMap = {};
    min.scriptMap = {};
    min.sandBoxMap = {};
    min.packages = sysPackages;

    if (min.instance.whatsappServiceKey !== null) {
      min.whatsAppDirectLine = new WhatsappDirectLine(
        min,
        min.botId,
        min.instance.whatsappBotKey,
        min.instance.whatsappServiceKey,
        min.instance.whatsappServiceNumber,
        min.instance.whatsappServiceUrl
      );
      await min.whatsAppDirectLine.setup(true);
    }
    else {
      const minBoot = GBServer.globals.minBoot as any;
      min.whatsAppDirectLine =
        new WhatsappDirectLine(
          min,
          min.botId,
          min.instance.webchatKey,
          minBoot.instance.whatsappServiceKey,
          minBoot.instance.whatsappServiceNumber,
          minBoot.instance.whatsappServiceUrl
        );
      await min.whatsAppDirectLine.setup(false);
    }

    min.userProfile = conversationState.createProperty('userProfile');
    const dialogState = conversationState.createProperty('dialogState');

    min.dialogs = new DialogSet(dialogState);
    min.dialogs.add(new TextPrompt('textPrompt'));
    min.dialogs.add(new ConfirmPrompt('confirmPrompt'));

    return { min, adapter, conversationState };
  }

  private async invokeLoadBot(appPackages: IGBPackage[], sysPackages: IGBPackage[], min: GBMinInstance) {
    await CollectionUtil.asyncForEach(sysPackages, async e => {
      await e.loadBot(min);
    });

    await CollectionUtil.asyncForEach(appPackages, async p => {
      p.sysPackages = sysPackages;
      await p.loadBot(min);
      if (p.getDialogs !== undefined) {
        const dialogs = await p.getDialogs(min);
        dialogs.forEach(dialog => {
          min.dialogs.add(new WaterfallDialog(dialog.id, dialog.waterfall));
        });
      }
    });

  }

  /**
   * Bot Service hook method.
   */
  private async receiver(
    adapter: BotFrameworkAdapter,
    req: any,
    res: any,
    conversationState: ConversationState,
    min: GBMinInstance,
    instance: any,
    appPackages: any[]
  ) {
    await adapter.processActivity(req, res, async context => {
      // Get loaded user state
      const step = await min.dialogs.createContext(context);
      step.context.activity.locale = 'pt-BR';

      try {

        const user = await min.userProfile.get(context, {});

        // First time processing.

        if (!user.loaded) {
          await min.conversationalService.sendEvent(min, step, 'loadInstance', {
            instanceId: instance.instanceId,
            botId: instance.botId,
            theme: instance.theme ? instance.theme : 'default.gbtheme',
            secret: instance.webchatKey
          });
          user.loaded = true;
          user.subjects = [];
          user.cb = undefined;


          if (context.activity.membersAdded !== undefined) {
            let sec = new SecService();
            const member = context.activity.membersAdded[0];

            const persistedUser = await sec.ensureUser(instance.instanceId, member.id,
              member.name, "", "web", member.name);

            const analytics = new AnalyticsService();

            user.systemUser = persistedUser;
            user.conversation = await analytics.createConversation(persistedUser);

          }

          await min.userProfile.set(step.context, user);

        }

        GBLog.info(
          `User>: ${context.activity.text} (${context.activity.type}, ${context.activity.name}, ${
          context.activity.channelId
          }, {context.activity.value})`
        );
        if (context.activity.type === 'conversationUpdate' && context.activity.membersAdded.length > 0) {
          const member = context.activity.membersAdded[0];
          if (member.name === min.instance.title) {
            GBLog.info(`Bot added to conversation, starting chat...`);
            await CollectionUtil.asyncForEach(appPackages, async e => {
              await e.onNewSession(min, step);
            });
            await step.beginDialog('/');
          } else {
            GBLog.info(`Member added to conversation: ${member.name}`);
          }

          // Processes messages.
        } else if (context.activity.type === 'message') {
          // Checks for /admin request.
          await this.processMessageActivity(context, min, step);

          // Processes events.
        } else if (context.activity.type === 'event') {
          // Empties dialog stack before going to the target.

          await this.processEventActivity(context, step);
        }
        await conversationState.saveChanges(context, true);
      } catch (error) {
        const msg = `ERROR: ${error.message} ${error.stack ? error.stack : ''}`;
        GBLog.error(msg);

        await min.conversationalService.sendText(min, step, Messages[step.context.activity.locale].very_sorry_about_error);
        await step.beginDialog('/ask', { isReturning: true });
      }
    });
  }

  private async processEventActivity(context, step: GBDialogStep) {
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
    } else if (context.activity.name === 'updateToken') {
      const token = context.activity.data;
      await step.beginDialog('/adminUpdateToken', { token: token });
    } else {
      await step.continueDialog();
    }
  }

  private async processMessageActivity(context, min: GBMinInstance, step: GBDialogStep) {

    if (process.env.PRIVACY_STORE_MESSAGES === "true") {

      // Adds message to the analytics layer.

      const analytics = new AnalyticsService();
      const user = await min.userProfile.get(context, {});
      analytics.createMessage(min.instance.instanceId,
        user.conversation, user.systemUser,
        context.activity.text);
    }

    // Checks for global exit kewywords cancelling any active dialogs.

    const globalQuit = (locale, utterance) => {
      return utterance.match(Messages.global_quit);
    }

    const isVMCall = Object.keys(min.scriptMap).find(key => min.scriptMap[key] === context.activity.text) !== undefined;

    const simpleLocale = context.activity.locale.substring(0, 2);
    const hasBadWord = wash.check(simpleLocale, context.activity.text);

    if (hasBadWord) {
      await step.beginDialog('/pleaseNoBadWords');
    } else if (isVMCall) {
      await GBMinService.callVM(context.activity.text, min, step);
    } else if (context.activity.text.charAt(0) === '/') {
      let text = context.activity.text;
      let parts = text.split(' ');
      let dialogName = parts[0];
      parts.splice(0, 1);
      let args = parts.join(' ');
      await step.beginDialog(dialogName, { args: args });

    } else if (globalQuit(step.context.activity.locale, context.activity.text)) { // TODO: Hard-code additional languages.
      await step.cancelAllDialogs();
      await min.conversationalService.sendText(min, step, Messages[step.context.activity.locale].canceled);
    } else if (context.activity.text === 'admin') {
      await step.beginDialog('/admin');

      // Checks for /menu JSON signature.
    } else if (context.activity.text.startsWith('{"title"')) {
      await step.beginDialog('/menu', JSON.parse(context.activity.text));
      // Otherwise, continue to the active dialog in the stack.
    } else {
      if (step.activeDialog !== undefined) {
        await step.continueDialog();
      } else {

        let query = context.activity.text;
        
        let locale = 'pt';
        if (process.env.TRANSLATOR_DISABLED !== "true"){
          locale = await AzureText.getLocale(min.instance.textAnalyticsKey,
          min.instance.textAnalyticsEndpoint, query);
        }

        let sec = new SecService();
        const member = step.context.activity.from;

        const user = await sec.ensureUser(min.instance.instanceId, member.id,
          member.name, "", "web", member.name);
        user.locale = locale;
        await user.save();

        query = await min.conversationalService.translate(
          min.instance.translatorKey,
          min.instance.translatorEndpoint,
          query,
          'pt');
        GBLog.info(`Translated text: ${query}.`)

        await step.beginDialog('/answer', {
          query: query
        });
      }
    }
  }

  public static async callVM(text: string, min: GBMinInstance, step: GBDialogStep) {
    const mainMethod = text;
    min.sandBoxMap[mainMethod][mainMethod].bind(min.sandBoxMap[mainMethod]);
    return await min.sandBoxMap[mainMethod][mainMethod](step);
  }
}
