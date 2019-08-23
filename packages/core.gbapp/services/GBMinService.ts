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
import { AutoSaveStateMiddleware, BotFrameworkAdapter, ConversationState, MemoryStorage, UserState } from 'botbuilder';
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
import { GBAnalyticsPackage } from '../../analytics.gblib';
import { GBCorePackage } from '../../core.gbapp';
import { GBCustomerSatisfactionPackage } from '../../customer-satisfaction.gbapp';
import { GBKBPackage } from '../../kb.gbapp';
import { AskDialogArgs } from '../../kb.gbapp/dialogs/AskDialog';
import { GBSecurityPackage } from '../../security.gblib';
import { GBWhatsappPackage } from '../../whatsapp.gblib';
import { Messages } from '../strings';
import { GBAdminPackage } from './../../admin.gbapp/index';
import { GBConfigService } from './GBConfigService';
import { GBDeployer } from './GBDeployer';
import { SecService } from '../../security.gblib/services/SecService';
import { isBreakOrContinueStatement } from 'typescript';

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
      GBServer.globals.server.use('/', express.static(urlJoin(GBDeployer.deployFolder, GBMinService.uiPackage, 'build')));
    }
    // Serves the bot information object via HTTP so clients can get
    // instance information stored on server.
    if (process.env.DISABLE_WEB !== 'true') {
      GBServer.globals.server.get('/instances/:botId', (req, res) => {
        (async () => {
          await this.handleGetInstanceFroClient(req, res);
        })();
      });
    }
    const url = '/webhooks/whatsapp';
    GBServer.globals.server.post(url, async (req, res) => {
      try {

        const id = req.body.messages[0].chatId.split('@')[0];
        const text = req.body.messages[0].body;
        if (req.body.messages[0].fromMe) {
          res.end();
          return; // Exit here.
        }

        const minBoot = GBServer.globals.bootInstance;
        const toSwitchMin = GBServer.globals.minInstances.filter(p => p.botId === text)[0];
        let activeMin = toSwitchMin ? toSwitchMin : minBoot;

        let sec = new SecService();
        let user = await sec.getUserFromPhone(id);

        if (user === null) {
          user = await sec.ensureUser(activeMin.instance.instanceId, id,
            activeMin.botId, id, "", "whatsapp", id, id);
          await (activeMin as any).whatsAppDirectLine.sendToDevice(id, `Olá! Seja bem-vinda(o)!\nMe chamo ${activeMin.instance.title}. Como posso ajudar?`);
          res.end();
        } else {
          // User wants to switch bots.
          if (toSwitchMin !== undefined) {
            await sec.updateCurrentBotId(id, text);
            await (activeMin as any).whatsAppDirectLine.sendToDevice(id, `Agora falando com ${activeMin.instance.title}...`);
            res.end();
          }
          else {
            activeMin = GBServer.globals.minInstances.filter(p => p.botId === user.currentBotId)[0];;
            (activeMin as any).whatsAppDirectLine.received(req, res);
          }
        }
      } catch (error) {
        GBLog.error(`Error on Whatsapp callback: ${error.message}`);
      }
    });

    await Promise.all(
      instances.map(async instance => {
        // Gets the authorization key for each instance from Bot Service.

        await this.mountBot(instance);
      })
    );
  }

  public async unmountBot(botId: string) {
    const url = `/api/messages/${botId}`;
    removeRoute(GBServer.globals.server, url);

    const uiUrl = `/${botId}`;
    removeRoute(GBServer.globals.server, uiUrl);

    GBServer.globals.minInstances = GBServer.globals.minInstances.filter(p => p.botId !== botId);
  }

  public async mountBot(instance: IGBInstance) {

    // Build bot adapter.
    const { min, adapter, conversationState } = await this.buildBotAdapter(instance, GBServer.globals.publicAddress, GBServer.globals.sysPackages);
    GBServer.globals.minInstances.push(min);

    // Install default VBA module.
    // this.deployer.deployPackage(min, 'packages/default.gbdialog');

    // Call the loadBot context.activity for all packages.
    this.invokeLoadBot(GBServer.globals.appPackages, GBServer.globals.sysPackages, min, GBServer.globals.server);

    // Serves individual URL for each bot conversational interface...
    const url = `/api/messages/${instance.botId}`;
    GBServer.globals.server.post(url, async (req, res) => {
      await this.receiver(adapter, req, res, conversationState, min, instance, GBServer.globals.appPackages);
    });
    GBLog.info(`GeneralBots(${instance.engineName}) listening on: ${url}.`);

    // Serves individual URL for each bot user interface.
    if (process.env.DISABLE_WEB !== 'true') {
      const uiUrl = `/${instance.botId}`;
      GBServer.globals.server.use(uiUrl, express.static(urlJoin(GBDeployer.deployFolder, GBMinService.uiPackage, 'build')));
      GBLog.info(`Bot UI ${GBMinService.uiPackage} accessible at: ${uiUrl}.`);
    }

    // Clients get redirected here in order to create an OAuth authorize url and redirect them to AAD.
    // There they will authenticate and give their consent to allow this app access to
    // some resource they own.
    this.handleOAuthRequests(GBServer.globals.server, min);

    // After consent is granted AAD redirects here.  The ADAL library
    // is invoked via the AuthenticationContext and retrieves an
    // access token that can be used to access the user owned resource.
    this.handleOAuthTokenRequests(GBServer.globals.server, min, instance);
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
        instance.authenticatorClientId,
        instance.authenticatorClientSecret,
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
        min.instance.authenticatorClientId
        }&redirect_uri=${urlJoin(min.instance.botEndpoint, min.instance.botId, 'token')}`;
      res.redirect(authorizationUrl);
    });
  }

  /**
   * Returns the instance object to clients requesting bot info.
   */
  private async handleGetInstanceFroClient(req: any, res: any) {
    let botId = req.params.botId;
    if (botId === '[default]' || botId === undefined) {
      botId = GBConfigService.get('BOT_ID');
    }
    const instance = await this.core.loadInstance(botId);
    if (instance !== null) {
      const webchatToken = await this.getWebchatToken(instance);
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
          secret: instance.webchatKey,
          speechToken: speechToken,
          conversationId: webchatToken.conversationId,
          authenticatorTenant: instance.authenticatorTenant,
          authenticatorClientId: instance.authenticatorClientId
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
      url: 'https://westus.api.cognitive.microsoft.com/sts/v1.0/issueToken',
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

  private async buildBotAdapter(instance: any, proxyAddress: string, sysPackages: IGBPackage[]) {
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
    min.botId = instance.botId;
    min.bot = adapter;
    min.userState = userState;
    min.core = this.core;
    min.conversationalService = this.conversationalService;
    min.adminService = this.adminService;
    min.instance = await this.core.loadInstance(min.botId);
    min.cbMap = {};
    min.scriptMap = {};
    min.sandBoxMap = {};
    min.packages = sysPackages;
    min.userProfile = conversationState.createProperty('userProfile');
    const dialogState = conversationState.createProperty('dialogState');

    min.dialogs = new DialogSet(dialogState);
    min.dialogs.add(new TextPrompt('textPrompt'));
    min.dialogs.add(new ConfirmPrompt('confirmPrompt'));

    return { min, adapter, conversationState };
  }

  private invokeLoadBot(appPackages: IGBPackage[], sysPackages: IGBPackage[], min: GBMinInstance, server: any) {
    let index = 0;
    sysPackages.forEach(e => {
      e.loadBot(min);
      index++;
    }, this);

    appPackages.forEach(p => {
      p.sysPackages = sysPackages;
      p.loadBot(min);
      if (p.getDialogs !== undefined) {
        const dialogs = p.getDialogs(min);
        dialogs.forEach(dialog => {
          min.dialogs.add(new WaterfallDialog(dialog.id, dialog.waterfall));
        });
      }
    }, this);
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
          await min.conversationalService.sendEvent(step, 'loadInstance', {
            instanceId: instance.instanceId,
            botId: instance.botId,
            theme: instance.theme ? instance.theme : 'default.gbtheme',
            secret: instance.webchatKey
          });
          user.loaded = true;
          user.subjects = [];
          user.cb = undefined;
          await min.userProfile.set(step.context, user);

          let sec = new SecService();
          const member = context.activity.membersAdded[0];

          await sec.ensureUser(instance.instanceId, member.id,
            min.botId, member.id, "", "web", member.name, member.id);
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
            appPackages.forEach(e => {
              e.onNewSession(min, step);
            });
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

        await step.context.sendActivity(Messages[step.context.activity.locale].very_sorry_about_error);
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
    // Direct script invoking by itent name.
    const globalQuit = (locale, utterance) => {
      return utterance.match(Messages[locale].global_quit);
    }


    const isVMCall = Object.keys(min.scriptMap).find(key => min.scriptMap[key] === context.activity.text) !== undefined;

    if (isVMCall) {
      const mainMethod = context.activity.text;

      min.sandBoxMap[mainMethod].context = context;
      min.sandBoxMap[mainMethod].step = step;
      min.sandBoxMap[mainMethod][mainMethod].bind(min.sandBoxMap[mainMethod]);
      await min.sandBoxMap[mainMethod][mainMethod]();
    } else if (context.activity.text.charAt(0) === '/') {
      await step.beginDialog(context.activity.text);

    } else if (globalQuit(step.context.activity.locale, context.activity.text)) {
      await step.cancelAllDialogs();
      await step.context.sendActivity(Messages[step.context.activity.locale].canceled);
    } else if (context.activity.text === 'admin') {
      await step.beginDialog('/admin');

      // Checks for /menu JSON signature.
    } else if (context.activity.text.startsWith('{"title"')) {
      await step.beginDialog('/menu', JSON.parse(context.activity.text));
      // Otherwise, continue to the active dialog in the stack.
    } else {
      const user = await min.userProfile.get(context, {});
      if (step.activeDialog !== undefined) {
        await step.continueDialog();
      } else {
        await step.beginDialog('/answer', {
          query: context.activity.text
        });
      }
    }
  }
}
