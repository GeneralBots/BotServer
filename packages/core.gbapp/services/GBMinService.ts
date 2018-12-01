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

const { DialogSet, TextPrompt } = require('botbuilder-dialogs');
const UrlJoin = require('url-join');
const express = require('express');
const logger = require('../../../src/logger');
const request = require('request-promise-native');
const AuthenticationContext = require('adal-node').AuthenticationContext;

import { AutoSaveStateMiddleware, BotFrameworkAdapter, ConversationState, MemoryStorage, UserState } from 'botbuilder';

import { GBMinInstance, IGBAdminService, IGBConversationalService, IGBCoreService, IGBPackage } from 'botlib';
import { GBAnalyticsPackage } from '../../analytics.gblib';
import { GBCorePackage } from '../../core.gbapp';
import { GBCustomerSatisfactionPackage } from '../../customer-satisfaction.gbapp';
import { GBKBPackage } from '../../kb.gbapp';
import { GBSecurityPackage } from '../../security.gblib';
import { GBWhatsappPackage } from '../../whatsapp.gblib';
import { GuaribasInstance } from '../models/GBModel';
import { Messages } from '../strings';
import { GBAdminPackage } from './../../admin.gbapp/index';
import { GBDeployer } from './GBDeployer';

/** Minimal service layer for a bot. */

export class GBMinService {
  public core: IGBCoreService;
  public conversationalService: IGBConversationalService;
  public adminService: IGBAdminService;
  public deployer: GBDeployer;

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
   * */

  public async buildMin(
    bootInstance: GuaribasInstance,
    server: any,
    appPackages: IGBPackage[],
    instances: GuaribasInstance[],
    deployer: GBDeployer
  ): Promise<GBMinInstance> {
    // Serves default UI on root address '/'.

    const uiPackage = 'default.gbui';
    server.use('/', express.static(UrlJoin(GBDeployer.deployFolder, uiPackage, 'build')));

    Promise.all(
      instances.map(async instance => {
        // Gets the authorization key for each instance from Bot Service.

        const webchatToken = await this.getWebchatToken(instance);

        // Serves the bot information object via HTTP so clients can get
        // instance information stored on server.

        server.get('/instances/:botId', (req, res) => {
          (async () => {
            // Returns the instance object to clients requesting bot info.

            let botId = req.params.botId;
            if (botId === '[default]') {
              botId = bootInstance.botId;
            }

            const instance = await this.core.loadInstance(botId);

            if (instance) {
              const speechToken = await this.getSTSToken(instance);
              let theme = instance.theme;
              if (!theme) {
                theme = 'default.gbtheme';
              }

              res.send(
                JSON.stringify({
                  instanceId: instance.instanceId,
                  botId: botId,
                  theme: theme,
                  secret: instance.webchatKey, // TODO: Use token.
                  speechToken: speechToken,
                  conversationId: webchatToken.conversationId,
                  authenticatorTenant: instance.authenticatorTenant,
                  authenticatorClientId: instance.authenticatorClientId
                })
              );
            } else {
              const error = `Instance not found: ${botId}.`;
              res.sendStatus(error);
              logger.error(error);
            }
          })();
        });

        // Build bot adapter.

        const { min, adapter, conversationState } = await this.buildBotAdapter(instance);

        // Install default VBA module.

        deployer.deployPackageFromLocalPath(min, 'packages/default.gbdialog');

        // Call the loadBot context.activity for all packages.

        this.invokeLoadBot(appPackages, min, server);

        // Serves individual URL for each bot conversational interface...

        const url = `/api/messages/${instance.botId}`;
        server.post(url, async (req, res) => {
          return await this.receiver(adapter, req, res, conversationState, min, instance, appPackages);
        });
        logger.info(`GeneralBots(${instance.engineName}) listening on: ${url}.`);

        // Serves individual URL for each bot user interface.

        const uiUrl = `/${instance.botId}`;
        server.use(uiUrl, express.static(UrlJoin(GBDeployer.deployFolder, uiPackage, 'build')));

        logger.info(`Bot UI ${uiPackage} accessible at: ${uiUrl}.`);
        const state = `${instance.instanceId}${Math.floor(Math.random() * 1000000000)}`;

        // Clients get redirected here in order to create an OAuth authorize url and redirect them to AAD.
        // There they will authenticate and give their consent to allow this app access to
        // some resource they own.
        server.get(`/${min.instance.botId}/auth`, function(req, res) {
          let authorizationUrl = UrlJoin(
            min.instance.authenticatorAuthorityHostUrl,
            min.instance.authenticatorTenant,
            '/oauth2/authorize'
          );
          authorizationUrl = `${authorizationUrl}?response_type=code&client_id=${
            min.instance.authenticatorClientId
          }&redirect_uri=${min.instance.botEndpoint}/${min.instance.botId}/token`;

          res.redirect(authorizationUrl);
        });

        // After consent is granted AAD redirects here.  The ADAL library
        // is invoked via the AuthenticationContext and retrieves an
        // access token that can be used to access the user owned resource.

        server.get(`/${min.instance.botId}/token`, async (req, res) => {
          const state = await min.adminService.getValue(min.instance.instanceId, 'AntiCSRFAttackState');

          if (req.query.state !== state) {
            const msg = 'WARNING: state field was not provided as anti-CSRF token';
            logger.error(msg);
            throw new Error(msg);
          }

          const authenticationContext = new AuthenticationContext(
            UrlJoin(min.instance.authenticatorAuthorityHostUrl, min.instance.authenticatorTenant)
          );

          const resource = 'https://graph.microsoft.com';

          authenticationContext.acquireTokenWithAuthorizationCode(
            req.query.code,
            UrlJoin(instance.botEndpoint, min.instance.botId, '/token'),
            resource,
            instance.authenticatorClientId,
            instance.authenticatorClientSecret,
            async (err, token) => {
              if (err) {
                const msg = `Error acquiring token: ${err}`;
                logger.error(msg);
                res.send(msg);
              } else {
                await this.adminService.setValue(instance.instanceId, 'refreshToken', token.refreshToken);
                await this.adminService.setValue(instance.instanceId, 'accessToken', token.accessToken);
                await this.adminService.setValue(instance.instanceId, 'expiresOn', token.expiresOn.toString());
                await this.adminService.setValue(instance.instanceId, 'AntiCSRFAttackState', null);

                res.redirect(min.instance.botEndpoint);
              }
            }
          );
        });
      })
    );
  }

  /**
   * Get Webchat key from Bot Service.
   *
   * @param instance The Bot instance.
   *
   */
  public async getWebchatToken(instance: any) {
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
  public async getSTSToken(instance: any) {
    // TODO: Make dynamic: https://CHANGE.api.cognitive.microsoft.com/sts/v1.0

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

  private async buildBotAdapter(instance: any) {
    const adapter = new BotFrameworkAdapter({
      appId: instance.marketplaceId,
      appPassword: instance.marketplacePassword
    });

    const storage = new MemoryStorage();
    const conversationState = new ConversationState(storage);
    const userState = new UserState(storage);
    adapter.use(new AutoSaveStateMiddleware(conversationState, userState));

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
    min.userProfile = conversationState.createProperty('userProfile');
    const dialogState = conversationState.createProperty('dialogState');

    min.dialogs = new DialogSet(dialogState);
    min.dialogs.add(new TextPrompt('textPrompt'));

    return { min, adapter, conversationState };
  }

  private invokeLoadBot(appPackages: any[], min: any, server: any) {
    const sysPackages = new Array<IGBPackage>();
    // NOTE: A semicolon is necessary before this line.
    [
      GBCorePackage,
      GBSecurityPackage,
      GBAdminPackage,
      GBKBPackage,
      GBAnalyticsPackage,
      GBCustomerSatisfactionPackage,
      GBWhatsappPackage
    ].forEach(sysPackage => {
      const p = Object.create(sysPackage.prototype) as IGBPackage;
      p.loadBot(min);
      sysPackages.push(p);
      if (sysPackage.name === 'GBWhatsappPackage') {
        const url = '/instances/:botId/whatsapp';
        server.post(url, (req, res) => {
          p.channel.received(req, res);
        });
      }
    }, this);

    appPackages.forEach(e => {
      e.sysPackages = sysPackages;
      e.loadBot(min);
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
    min: any,
    instance: any,
    appPackages: any[]
  ) {
    return await adapter.processActivity(req, res, async context => {
      // Get loaded user state
      const state = await conversationState.get(context);
      const step = await min.dialogs.createContext(context, state);
      step.context.activity.locale = 'en-US'; // TODO: Make dynamic.

      try {
        const user = await min.userProfile.get(context, {});

        if (!user.loaded) {
          await min.conversationalService.sendEvent(step, 'loadInstance', {
            instanceId: instance.instanceId,
            botId: instance.botId,
            theme: instance.theme ? instance.theme : 'default.gbtheme',
            secret: instance.webchatKey
          });
          user.loaded = true;
          user.subjects = [];
          user.cb = null;
          await min.userProfile.set(step.context, user);
        }

        logger.info(
          `User>: ${context.activity.text} (${context.activity.type}, ${context.activity.name}, ${
            context.activity.channelId
          }, {context.activity.value})`
        );
        if (context.activity.type === 'conversationUpdate' && context.activity.membersAdded.length > 0) {
          const member = context.activity.membersAdded[0];
          if (member.name === 'GeneralBots') {
            logger.info(`Bot added to conversation, starting chat...`);
            appPackages.forEach(e => {
              e.onNewSession(min, step);
            });
            // Processes the root dialog.

            await step.beginDialog('/');
          } else {
            logger.info(`Member added to conversation: ${member.name}`);
          }

          // Processes messages.
        } else if (context.activity.type === 'message') {
          // Checks for /admin request.
          if (context.activity.text === 'vba') {
            min.sandbox.context = context;
            min.sandbox.step = step;
            min.sandbox['bot'].bind(min.sandbox);
            await min.sandbox['bot']();
          } else if (context.activity.text === 'admin') {
            await step.beginDialog('/admin');

            // Checks for /menu JSON signature.
          } else if (context.activity.text.startsWith('{"title"')) {
            await step.beginDialog('/menu', {
              data: JSON.parse(context.activity.text)
            });

            // Otherwise, continue to the active dialog in the stack.
          } else {
            const user = await min.userProfile.get(context, {});

            if (step.activeDialog || user.dialog) {
              await step.continueDialog();
            } else {
              await step.beginDialog('/answer', {
                query: context.activity.text
              });
            }
          }

          // Processes events.
        } else if (context.activity.type === 'event') {
          // Empties dialog stack before going to the target.

          await step.endAll();

          if (context.activity.name === 'whoAmI') {
            await step.beginDialog('/whoAmI');
          } else if (context.activity.name === 'showSubjects') {
            await step.beginDialog('/menu');
          } else if (context.activity.name === 'giveFeedback') {
            await step.beginDialog('/feedback', {
              fromMenu: true
            });
          } else if (context.activity.name === 'showFAQ') {
            await step.beginDialog('/faq');
          } else if (context.activity.name === 'answerEvent') {
            await step.beginDialog('/answerEvent', {
              questionId: (context.activity as any).data,
              fromFaq: true
            });
          } else if (context.activity.name === 'quality') {
            await step.beginDialog('/quality', {
              score: (context.activity as any).data
            });
          } else if (context.activity.name === 'updateToken') {
            const token = (context.activity as any).data;
            await step.beginDialog('/adminUpdateToken', { token: token });
          } else {
            await step.continueDialog();
          }
        }
        await conversationState.saveChanges(context, true);
      } catch (error) {
        const msg = `ERROR: ${error.message} ${error.stack ? error.stack : ''}`;
        logger.error(msg);

        await step.context.sendActivity(Messages[step.context.activity.locale].very_sorry_about_error);
        await step.beginDialog('/ask', { isReturning: true });
      }
    });
  }
}
