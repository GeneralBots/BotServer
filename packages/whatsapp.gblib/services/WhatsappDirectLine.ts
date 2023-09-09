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

import mime from 'mime-types';
import urlJoin from 'url-join';
import SwaggerClient from 'swagger-client';
import Path from 'path';
import Fs from 'fs';
import { GBError, GBLog, GBMinInstance, GBService, IGBPackage } from 'botlib';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { GBServer } from '../../../src/app.js';
import { GBConversationalService } from '../../core.gbapp/services/GBConversationalService.js';
import { SecService } from '../../security.gbapp/services/SecService.js';
import { Messages } from '../strings.js';
import { GuaribasUser } from '../../security.gbapp/models/index.js';
import { GBMinService } from '../../core.gbapp/services/GBMinService.js';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService.js';
import qrcode from 'qrcode-terminal';
import express from 'express';
import { GBSSR } from '../../core.gbapp/services/GBSSR.js';
import pkg from 'whatsapp-web.js';
import { DialogKeywords } from '../../basic.gblib/services/DialogKeywords.js';
import { ChatServices } from '../../gpt.gblib/services/ChatServices.js';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import e from 'express';
const { List, Buttons, Client, MessageMedia } = pkg;

/**
 * Support for Whatsapp.
 */
export class WhatsappDirectLine extends GBService {
  public static conversationIds = {};
  public static botsByNumber = {};
  public static mobiles = {};
  public static phones = {};
  public static chatIds = {};
  public static usernames = {};
  public static state = {}; // 2: Waiting, 3: MessageArrived.
  public static lastMessage = {}; // 2: Waiting, 3: MessageArrived.
  public static botGroups = {};

  public pollInterval = 3000;
  public directLineClientName = 'DirectLineClient';

  public directLineClient: any;
  public whatsappServiceKey: string;
  public whatsappServiceNumber: string;
  public whatsappServiceUrl: string;
  public botId: string;
  public min: GBMinInstance;
  private directLineSecret: string;
  private locale: string = 'pt-BR';
  provider: any;
  INSTANCE_URL = 'https://api.maytapi.com/api';
  private customClient: any;
  private browserWSEndpoint: any;
  private groupId;

  constructor(
    min: GBMinInstance,
    botId,
    directLineSecret,
    whatsappServiceKey,
    whatsappServiceNumber,
    whatsappServiceUrl,
    groupId
  ) {
    super();

    this.min = min;
    this.botId = botId;
    this.directLineSecret = directLineSecret;
    this.whatsappServiceKey = whatsappServiceKey;
    this.whatsappServiceNumber = whatsappServiceNumber;
    this.whatsappServiceUrl = whatsappServiceUrl;
    this.provider =
      whatsappServiceKey === 'internal'
        ? 'GeneralBots'
        : whatsappServiceNumber.indexOf(';') > -1
        ? 'maytapi'
        : whatsappServiceKey !== 'internal'
        ? 'graphapi'
        : 'chatapi';
    this.groupId = groupId;
  }

  public static async asyncForEach(array, callback) {
    for (let index = 0; index < array.length; index++) {
      await callback(array[index], index, array);
    }
  }

  public async setup(setUrl: boolean) {
    const client = await new SwaggerClient({
      spec: JSON.parse(Fs.readFileSync('directline-3.0.json', 'utf8')),
      requestInterceptor: req => {
        req.headers['Authorization'] = `Bearer ${this.min.instance.whatsappBotKey}`;
      }
    });
    this.directLineClient = client;

    let url: string;
    let body: any;
    let options: any;

    switch (this.provider) {
      case 'GeneralBots':
        const minBoot = GBServer.globals.minBoot;
        // Initialize the browser using a local profile for each bot.
        const path = DialogKeywords.getGBAIPath(this.min.botId);
        const localName = Path.join('work', path, 'profile');
        const createClient = () => {
          const client = (this.customClient = new Client({
            puppeteer: GBSSR.preparePuppeteer(localName)
          }));
          client.on(
            'message',
            (async message => {
              await this.WhatsAppCallback(message, null);
            }).bind(this)
          );
          client.on(
            'qr',
            (async qr => {
              const adminNumber = this.min.core.getParam(this.min.instance, 'Bot Admin Number', null);
              const adminEmail = this.min.core.getParam(this.min.instance, 'Bot Admin E-mail', null);
              // Sends QR Code to boot bot admin.
              const msg = `Please, scan QR Code with for bot ${this.botId}.`;
              GBLog.info(msg);
              qrcode.generate(qr, { small: true, scale: 0.5 });
              // While handling other bots uses boot instance of this class to send QR Codes.
              // const s = new DialogKeywords(min., null, null, null);
              // const qrBuf = await s.getQRCode(qr);
              // const localName = Path.join('work', gbaiName, 'cache', `qr${GBAdminService.getRndReadableIdentifier()}.png`);
              // fs.writeFileSync(localName, qrBuf);
              // const url = urlJoin(
              //   GBServer.globals.publicAddress,
              //   this.min.botId,
              //   'cache',
              //   Path.basename(localName)
              // );
              // GBServer.globals.minBoot.whatsAppDirectLine.sendFileToDevice(adminNumber, url, Path.basename(localName), msg);
              // s.sendEmail(adminEmail, `Check your WhatsApp for bot ${this.botId}`, msg);
            }).bind(this)
          );
          client.on('authenticated', async () => {
            GBLog.verbose(`GBWhatsApp: QR Code authenticated for ${this.botId}.`);
          });
          client.on('ready', async () => {
            GBLog.verbose(`GBWhatsApp: Emptying chat list for ${this.botId}...`);
            // TODO: await client.pupPage['minimize']();
            // Keeps the chat list cleaned.
            const chats = await client.getChats();
            await CollectionUtil.asyncForEach(chats, async chat => {
              const sleep = ms => {
                return new Promise(resolve => {
                  setTimeout(resolve, ms);
                });
              };
              const wait = Math.floor(Math.random() * 5000) + 1000;
              await sleep(wait);
              if (chat.isGroup) {
                // await chat.clearMessages();
              } else if (!chat.pinned) {
                // await chat.delete();
              }
            });
          });
          client.initialize();
        };
        if (setUrl) {
          createClient.bind(this)();
        } else {
          this.customClient = minBoot.whatsAppDirectLine.customClient;
        }
        setUrl = false;

        break;
      case 'chatapi':
        options = {
          method: 'POST',
          url: urlJoin(this.whatsappServiceUrl, 'webhook'),
          timeout: 10000,
          qs: {
            token: this.whatsappServiceKey,
            webhookUrl: `${GBServer.globals.publicAddress}/webhooks/whatsapp/${this.botId}`,
            set: true
          },
          headers: {
            'cache-control': 'no-cache'
          }
        };
        break;
      case 'official':
        url = urlJoin(this.whatsappServiceUrl, 'webhook');
        options = {
          method: 'POST',
          url: url,
          timeout: 10000,
          qs: {
            token: this.whatsappServiceKey,
            webhookUrl: `${GBServer.globals.publicAddress}/webhooks/whatsapp/${this.botId}`,
            set: true
          },
          headers: {
            'cache-control': 'no-cache'
          }
        };

        break;
      case 'maytapi':
        const phoneId = this.whatsappServiceNumber.split(';')[0];
        let productId = this.whatsappServiceNumber.split(';')[1];
        url = `${this.INSTANCE_URL}/${productId}/${phoneId}/config`;
        body = {
          webhook: `${GBServer.globals.publicAddress}/webhooks/whatsapp/${this.botId}`,
          ack_delivery: false
        };
        WhatsappDirectLine.phones[phoneId] = this.botId;

        options = {
          url: url,
          method: 'POST',
          body: body,
          headers: {
            'x-maytapi-key': this.whatsappServiceKey,
            'Content-Type': 'application/json'
          },
          json: true
        };
        break;
    }

    if (setUrl && options && this.whatsappServiceUrl) {
      GBServer.globals.server.use(`/audios`, express.static('work'));

      if (options) {
        try {
          const response: Response = await fetch(url, options);
        } catch (error) {
          GBLog.error(`Error initializing 3rd party Whatsapp provider(1) ${error.message}`);
        }
      }
    }
  }

  public async resetConversationId(botId: string, number: number, group = '') {
    WhatsappDirectLine.conversationIds[botId + number + group] = undefined;
  }

  public async check() {
    switch (this.provider) {
      case 'GeneralBots':
        const state = await this.customClient.getState();
        return state === 'CONNECTED';

      default:
        GBLog.verbose(`GBWhatsapp: Checking server...`);
        let url = urlJoin(this.whatsappServiceUrl, 'status') + `?token=${this.min.instance.whatsappServiceKey}`;
        const options = {
          url: url,
          method: 'GET'
        };

        const res = await fetch(url, options);
        const json = await res.json();
        return json['accountStatus'] === 'authenticated';
    }
  }

  public static providerFromRequest(req: any) {
    return req.body.messages ? 'chatapi' : req.body.message ? 'maytapi' : req.body.message ? 'graphapi' : 'GeneralBots';
  }

  public async received(req, res) {
    const provider = WhatsappDirectLine.providerFromRequest(req);

    let message, to, from, fromName, text: string;
    let group = '';
    let answerText = null;
    let attachments = null;

    switch (provider) {
      case 'GeneralBots':
        message = req;
        to = message.to.endsWith('@g.us') ? message.to.split('@')[0] : message.to.split('@')[0];
        const newThis = WhatsappDirectLine.botsByNumber[to];

        // If there is a number specified, checks if it
        // is related to a custom bot and reroutes immediately.

        if (newThis && newThis.min.botId !== GBServer.globals.minBoot.botId) {
          await newThis.received(req, res);

          return;
        }

        text = message.body;
        from = message.from.endsWith('@g.us') ? message.author.split('@')[0] : message.from.split('@')[0];
        fromName = message._data.notifyName;

        if (message.hasMedia) {
          const base64Image = await message.downloadMedia();

          let buf: any = Buffer.from(base64Image.data, 'base64');
          const gbaiName = DialogKeywords.getGBAIPath(this.min.botId);
          const localName = Path.join(
            'work',
            gbaiName,
            'cache',
            `tmp${GBAdminService.getRndReadableIdentifier()}.docx`
          );
          Fs.writeFileSync(localName, buf, { encoding: null });
          const url = urlJoin(GBServer.globals.publicAddress, this.min.botId, 'cache', Path.basename(localName));

          attachments = [];
          attachments.push({
            name: `${new Date().toISOString().replace(/\:/g, '')}.${mime.extension(base64Image.mimetype)}`,
            noName: true,
            contentType: base64Image.mimetype,
            contentUrl: url
          });
        }

        break;

      case 'chatapi':
        message = req.body.messages[0];
        text = message.body;
        from = req.body.messages[0].author.split('@')[0];
        fromName = req.body.messages[0].senderName;

        if (message.type !== 'chat') {
          attachments = [];
          attachments.push({
            name: 'uploaded',
            contentType: 'application/octet-stream',
            contentUrl: message.body
          });
        }

        if (req.body.messages[0].fromMe) {
          res.end();

          return; // Exit here.
        }

        break;
      case 'graphapi':
        break;

      case 'maytapi':
        message = req.body.message;
        text = message.text;
        from = req.body.user.phone;
        fromName = req.body.user.name;

        if (req.body.message.fromMe) {
          res.end();

          return; // Exit here.
        }
        break;
    }

    text = text.replace(/\@\d+ /gi, '');
    GBLog.info(`GBWhatsapp: RCV ${from}(${fromName}): ${text})`);

    let botGroupID = WhatsappDirectLine.botGroups[this.min.botId];
    let botShortcuts = this.min.core.getParam<string>(this.min.instance, 'WhatsApp Group Shortcuts', null);
    if (!botShortcuts) {
      botShortcuts = new Array();
    } else {
      botShortcuts = botShortcuts.split(' ');
    }

    if (provider === 'chatapi') {
      if (message.chatName.charAt(0) !== '+') {
        group = message.chatName;
      }
    } else if (provider === 'GeneralBots') {
      if (message.from.endsWith('@g.us')) {
        group = message.from;
      }
    }

    if (group) {
      const parts = text.split(' ');

      // Bot name must be specified on config.

      if (botGroupID === group) {
        // Shortcut has been mentioned?

        let found = false;
        parts.forEach(e1 => {
          botShortcuts.forEach(e2 => {
            if (e1 === e2 && !found) {
              found = true;
              text = text.replace(e2, '');
            }
          });

          // Verify if it is a group cache answer.

          const questions = this.min['groupCache'];
          if (questions && questions.length > 0) {
            questions.forEach(q => {
              if (q.content === e1 && !found) {
                const answer = this.min.kbService['getAnswerById'](this.min.instance.instanceId, q.answerId);
                answerText = answer.content;

                answerText = answerText.replace(/\$username/gi, fromName);
              }
            });
          }

          // Ignore group messages without the mention to Bot.

          let smsServiceNumber = this.min.core.getParam<string>(this.min.instance, 'whatsappServiceNumber', null);
          if (smsServiceNumber && !answerText) {
            smsServiceNumber = smsServiceNumber.replace('+', '');
            if (!message.body.startsWith('@' + smsServiceNumber)) {
              return;
            }
          }
        });
      }
    }

    const botId = this.min.instance.botId;
    const state = WhatsappDirectLine.state[botId + from];
    if (state) {
      GBLog.verbose(`BASIC: Continuing HEAR from WhatsApp...`);
      WhatsappDirectLine.state[botId + from] = null;
      await state.promise(null, text);

      return; // Exit here.
    }

    // Processes .gbapp message interception.

    await CollectionUtil.asyncForEach(this.min.appPackages, async (e: IGBPackage) => {
      await e.onExchangeData(this.min, 'whatsappMessage', { from, fromName });
    });

    const sec = new SecService();
    const user = await sec.ensureUser(this.min.instance.instanceId, from, fromName, '', 'whatsapp', fromName, null);
    const locale = user.locale ? user.locale : 'pt';

    if (answerText) {
      await this.sendToDeviceEx(user.userSystemId, answerText, locale, null);

      return; // Exit here.
    }

    if (message.type === 'ptt') {
      let url = provider ? message.body : message.text;
      if (process.env.AUDIO_DISABLED !== 'true') {
        const options = {
          url: url,
          method: 'GET',
          encoding: 'binary'
        };

        const res = await fetch(url, options);
        const buf = Buffer.from(await res.arrayBuffer());
        text = await GBConversationalService.getTextFromAudioBuffer(
          this.min.instance.speechKey,
          this.min.instance.cloudLocation,
          buf,
          locale
        );
      } else {
        await this.sendToDevice(user.userSystemId, `No momento estou apenas conseguindo ler mensagens de texto.`, null);
      }
    }

    const conversationId = WhatsappDirectLine.conversationIds[botId + from + group];
    const client = await this.directLineClient;
    WhatsappDirectLine.lastMessage[botId + from] = message;

    // Check if this message is from a Human Agent itself.

    if (user.agentMode === 'self') {
      // Check if there is someone being handled by this Human Agent.

      const manualUser = await sec.getUserFromAgentSystemId(from);
      if (manualUser === null) {
        await sec.updateHumanAgent(from, this.min.instance.instanceId, null);
      } else {
        const agent = await sec.getUserFromSystemId(user.agentSystemId);

        const cmd = '/reply ';
        if (text.startsWith(cmd)) {
          const filename = text.substr(cmd.length);
          const message = await this.min.kbService.getAnswerTextByMediaName(this.min.instance.instanceId, filename);

          if (message === null) {
            await this.sendToDeviceEx(
              user.userSystemId,
              `File ${filename} not found in any .gbkb published. Check the name or publish again the associated .gbkb.`,
              locale,
              null
            );
          } else {
            await this.min.conversationalService.sendMarkdownToMobile(this.min, null, user.userSystemId, message);
          }
        } else if (text === '/qt') {
          // https://github.com/GeneralBots/BotServer/issues/307

          await this.sendToDeviceEx(
            manualUser.userSystemId,
            Messages[this.locale].notify_end_transfer(this.min.instance.botId),
            locale,
            null
          );

          if (user.agentSystemId.indexOf('@') !== -1) {
            // Agent is from Teams.
            await this.min.conversationalService['sendOnConversation'](
              this.min,
              agent,
              Messages[this.locale].notify_end_transfer(this.min.instance.botId)
            );
          } else {
            await this.sendToDeviceEx(
              user.agentSystemId,
              Messages[this.locale].notify_end_transfer(this.min.instance.botId),
              locale,
              null
            );
          }
          await sec.updateHumanAgent(manualUser.userSystemId, this.min.instance.instanceId, null);
          await sec.updateHumanAgent(user.agentSystemId, this.min.instance.instanceId, null);
        } else {
          GBLog.info(`HUMAN AGENT (${manualUser.agentSystemId}) TO USER ${manualUser.userSystemId}: ${text}`);
          await this.sendToDeviceEx(manualUser.userSystemId, `AGENT: *${text}*`, locale, null);
        }
      }
    } else if (user.agentMode === 'human') {
      const agent = await sec.getUserFromSystemId(user.agentSystemId);
      if (text === '/t') {
        await this.sendToDeviceEx(
          user.userSystemId,
          `Você já está sendo atendido por ${agent.userSystemId}.`,
          locale,
          null
        );
      } else if (text === '/qt' || GBMinService.isGlobalQuitUtterance(locale, text)) {
        await this.endTransfer(from, locale, user, agent, sec);
      } else {
        GBLog.info(`USER (${from}) TO AGENT ${agent.userSystemId}: ${text}`);

        const prompt = `the person said: ${text}. what can I tell her?`;
        const answer = await ChatServices.continue(this.min, prompt, 0);
        text = `${text} \n\nGeneral Bots: ${answer}`;

        if (user.agentSystemId.indexOf('@') !== -1) {
          // Agent is from Teams or Google Chat.
          await this.min.conversationalService['sendOnConversation'](this.min, agent, text);
        } else {
          await this.sendToDeviceEx(
            user.agentSystemId,
            `Bot: ${this.min.instance.botId}\n${from}: ${text}`,
            locale,
            null
          );
        }
      }
    } else if (user.agentMode === 'bot' || user.agentMode === null || user.agentMode === undefined) {
      if (WhatsappDirectLine.conversationIds[botId + from + group] === undefined) {
        GBLog.info(`GBWhatsapp: Starting new conversation on Bot.`);
        const response = await client.apis.Conversations.Conversations_StartConversation();
        const generatedConversationId = response.obj.conversationId;

        WhatsappDirectLine.conversationIds[botId + from + group] = generatedConversationId;
        if (provider === 'GeneralBots') {
          WhatsappDirectLine.chatIds[generatedConversationId] = message.from;
        }
        WhatsappDirectLine.mobiles[generatedConversationId] = from;
        WhatsappDirectLine.usernames[from] = fromName;
        WhatsappDirectLine.chatIds[generatedConversationId] = message.chatId;

        this.pollMessages(client, generatedConversationId, from, fromName);
        this.inputMessage(client, generatedConversationId, text, from, fromName, group, attachments);
      } else {
        this.inputMessage(client, conversationId, text, from, fromName, group, attachments);
      }
    } else {
      GBLog.warn(`Inconsistencty found: Invalid agentMode on User Table: ${user.agentMode}`);
    }

    if (res) {
      res.end();
    }
  }

  private async endTransfer(id: string, locale: string, user: GuaribasUser, agent: GuaribasUser, sec: SecService) {
    await this.sendToDeviceEx(id, Messages[this.locale].notify_end_transfer(this.min.instance.botId), locale, null);

    if (user.agentSystemId.indexOf('@') !== -1) {
      // Agent is from Teams.

      await this.min.conversationalService['sendOnConversation'](
        this.min,
        agent,
        Messages[this.locale].notify_end_transfer(this.min.instance.botId)
      );
    } else {
      await this.sendToDeviceEx(
        user.agentSystemId,
        Messages[this.locale].notify_end_transfer(this.min.instance.botId),
        locale,
        null
      );
    }

    await sec.updateHumanAgent(id, this.min.instance.instanceId, null);
  }

  public inputMessage(client, conversationId: string, text: string, from, fromName: string, group, attachments: File) {
    try {
      return client.apis.Conversations.Conversations_PostActivity({
        conversationId: conversationId,
        activity: {
          textFormat: 'plain',
          text: text,
          type: 'message',
          mobile: from,
          group: group,
          attachments: attachments,

          // Use from container to transport information to GBMinService.receiver.

          from: {
            id: from,
            name: fromName,
            channelIdEx: 'whatsapp',
            group: group
          },
          replyToId: from
        }
      });
    } catch (e) {
      GBLog.error(e);
    }
  }

  public pollMessages(client, conversationId, from, fromName) {
    GBLog.info(`GBWhatsapp: Starting message polling(${from}, ${conversationId}).`);

    let watermark: any;

    const worker = async () => {
      try {
        const response = await client.apis.Conversations.Conversations_GetActivities({
          conversationId: conversationId,
          watermark: watermark
        });
        watermark = response.obj.watermark;
        await this.printMessages(response.obj.activities, conversationId, from, fromName);
      } catch (err) {
        GBLog.error(
          `Error calling printMessages on Whatsapp channel ${err.data === undefined ? err : err.data} ${
            err.errObj ? err.errObj.message : ''
          }`
        );
      }
    };
    setInterval(worker, this.pollInterval);
  }

  public async printMessages(activities, conversationId, from, fromName) {
    if (activities && activities.length) {
      // Ignore own messages.

      activities = activities.filter(m => m.from.id === this.botId && m.type === 'message');

      if (activities.length) {
        // Print other messages.

        await WhatsappDirectLine.asyncForEach(activities, async activity => {
          await this.printMessage(activity, conversationId, from, fromName);
        });
      }
    }
  }

  public async printMessage(activity, conversationId, to, toName) {
    let output = '';

    if (activity.text) {
      GBLog.info(`GBWhatsapp: SND ${to}(${toName}): ${activity.text}`);
      output = activity.text;
    }

    if (activity.attachments) {
      await CollectionUtil.asyncForEach(activity.attachments, async attachment => {
        switch (attachment.contentType) {
          case 'application/vnd.microsoft.card.hero':
            output += `\n${this.renderHeroCard(attachment)}`;
            break;

          case 'image/png':
            await this.sendFileToDevice(to, attachment.contentUrl, attachment.name, attachment.name, 0);

            return;

          default:
            GBLog.info(`Unknown content type: ${attachment.contentType}`);
        }
      });
    }

    await this.sendToDevice(to, output, conversationId);
  }

  public renderHeroCard(attachment) {
    return `${attachment.content.title} - ${attachment.content.text}`;
  }

  public async sendFileToDevice(to, url, filename, caption, chatId) {
    let options;
    switch (this.provider) {
      case 'GeneralBots':
        const attachment = await MessageMedia.fromUrl(url);
        to = to.replace('+', '');
        if (to.indexOf('@') == -1) {
          if (to.length == 18) {
            to = to + '@g.us';
          } else {
            to = to + '@c.us';
          }
        }

        await this.customClient.sendMessage(to, attachment, { caption: caption });
        break;

      case 'chatapi':
        options = {
          method: 'POST',
          url: urlJoin(this.whatsappServiceUrl, 'sendFile'),
          qs: {
            token: this.whatsappServiceKey
          },
          json: true,
          body: {
            phone: to,
            body: url,
            filename: filename,
            caption: caption
          },
          headers: {
            'cache-control': 'no-cache'
          }
        };

        break;
      case 'maytapi':
        let contents = 0;
        let body = {
          to_number: to,
          type: 'media',
          message: url,
          text: caption
        };

        let phoneId = this.whatsappServiceNumber.split(';')[0];
        let productId = this.whatsappServiceNumber.split(';')[1];

        options = {
          url: `${this.INSTANCE_URL}/${productId}/${phoneId}/sendMessage`,
          method: 'post',
          json: true,
          body,
          headers: {
            'Content-Type': 'application/json',
            'x-maytapi-key': this.whatsappServiceKey
          }
        };

        break;

      case 'graphapi':
        url = `https://graph.facebook.com/v15.0/${phoneId}/messages`;
        options = {
          method: 'POST',
          timeout: 10000,
          headers: {
            token: `Bearer `,
            'Content-Type': 'application/json'
          },
          body: {
            messaging_product: 'whatsapp',
            recipient_type: 'individual',
            to: phoneId
          }
        };
    }

    if (options) {
      try {
        // tslint:disable-next-line: await-promise
        const result = await fetch(url, options);
        GBLog.info(`File ${url} sent to ${to}: ${result}`);
      } catch (error) {
        GBLog.error(`Error sending file to Whatsapp provider ${error.message}`);
      }
    }
  }

  public async sendAudioToDevice(to, url, chatId) {
    let options;
    switch (this.provider) {
      case 'GeneralBots':
        const attachment = MessageMedia.fromUrl(url);
        await this.customClient.sendMessage(to, attachment);

        break;

      case 'chatapi':
        options = {
          method: 'POST',
          url: urlJoin(this.whatsappServiceUrl, 'sendPTT'),
          qs: {
            token: this.whatsappServiceKey,
            phone: chatId ? null : to,
            chatId: chatId,
            body: url
          },
          headers: {
            'cache-control': 'no-cache'
          }
        };

        break;
      case 'maytapi':
        throw GBError.create('Sending audio in Maytapi not supported.');
    }

    if (options) {
      try {
        const result = await fetch(url, options);
        GBLog.info(`Audio ${url} sent to ${to}: ${result}`);
      } catch (error) {
        GBLog.error(`Error sending audio message to Whatsapp provider ${error.message}`);
      }
    }
  }

  public async sendTextAsAudioToDevice(to, msg: string, chatId) {
    const url = await GBConversationalService.getAudioBufferFromText(msg);

    await this.sendFileToDevice(to, url, 'Audio', msg, chatId);
  }

  public async sendToDevice(to: string, msg: string, conversationId) {
    const cmd = '/audio ';
    let url;
    let chatId = WhatsappDirectLine.chatIds[conversationId];

    if (typeof msg !== 'object' && msg.startsWith(cmd)) {
      msg = msg.substr(cmd.length);

      return await this.sendTextAsAudioToDevice(to, msg, chatId);
    } else {
      let options;

      switch (this.provider) {
        case 'GeneralBots':
          if (to.indexOf('@') == -1) {
            if (to.length == 18) {
              to = to + '@g.us';
            } else {
              to = to + '@c.us';
            }
          }
          await this.customClient.sendMessage(to, msg);

          break;

        case 'chatapi':
          options = {
            method: 'POST',
            url: urlJoin(this.whatsappServiceUrl, 'message'),
            qs: {
              token: this.whatsappServiceKey,
              phone: chatId ? null : to,
              chatId: chatId,
              body: msg
            },
            headers: {
              'cache-control': 'no-cache'
            }
          };
          break;
        case 'maytapi':
          let phoneId = this.whatsappServiceNumber.split(';')[0];
          let productId = this.whatsappServiceNumber.split(';')[1];
          url = `${this.INSTANCE_URL}/${productId}/${phoneId}/sendMessage`;

          options = {
            method: 'post',
            json: true,
            body: { type: 'text', message: msg, to_number: to },
            headers: {
              'Content-Type': 'application/json',
              'x-maytapi-key': this.whatsappServiceKey
            }
          };
          break;
        case 'graphapi':
          break;
      }

      if (options) {
        try {
          GBLog.info(`Message [${msg}] is being sent to ${to}...`);
          await fetch(url, options);
        } catch (error) {
          GBLog.error(`Error sending message to Whatsapp provider ${error.message}`);
        }
      }
    }
  }

  public async sendToDeviceEx(to, text, locale, conversationId) {
    text = await this.min.conversationalService.translate(this.min, text, locale);
    await this.sendToDevice(to, text, conversationId);
  }

  private async WhatsAppCallback(req, res) {
    try {
      if (req.body && req.body.webhook) {
        res.status(200);
        res.end();

        return;
      }

      let provider = GBMinService.isChatAPI(req, res);
      let id;
      let senderName;
      let botId;
      let text;

      switch (provider) {
        case 'GeneralBots':
          // Ignore E2E messages and status updates.

          if ((req.type && req.type === 'e2e_notification') || req.isStatus) {
            return;
          }

          id = req.from.split('@')[0];
          senderName = req._data.notifyName;
          text = req.body;
          botId = this.botId;
          break;

        case 'chatapi':
          if (req.body.ack) {
            res.status(200);
            res.end();

            return;
          }
          if (req.body.messages[0].fromMe) {
            res.end();

            return; // Exit here.
          }
          id = req.body.messages[0].author.split('@')[0];
          senderName = req.body.messages[0].senderName;
          text = req.body.messages[0].body;
          botId = req.params.botId;
          if (botId === '[default]' || botId === undefined) {
            botId = GBConfigService.get('BOT_ID');
          }
          break;
        case 'maytapi':
          if (req.body.type !== 'message') {
            res.status(200);
            res.end();

            return;
          }
          if (req.body.message.fromMe) {
            res.end();

            return; // Exit here.
          }
          id = req.body.user.phone;
          senderName = req.body.user.name;
          text = req.body.message.text;

          botId = WhatsappDirectLine.phones[req.body.phoneId];
          break;
      }

      const sec = new SecService();
      let user = await sec.getUserFromSystemId(id);

      // Tries to find if user wants to switch bots.

      let toSwitchMin = GBServer.globals.minInstances.filter(
        p => p.instance.botId.toLowerCase() === text.toLowerCase()
      )[0];

      
      GBLog.info(`A WhatsApp mobile requested instance for: ${botId}.`);

      let urlMin: any = GBServer.globals.minInstances.filter(p => p.instance.botId === botId)[0];

      const botNumber = urlMin ? urlMin.core.getParam(urlMin.instance, 'Bot Number', null) : null;
      if (botNumber && GBServer.globals.minBoot.botId !== urlMin.botId) {
        GBLog.info(`${user.userSystemId} fixed by bot number talked to: ${botId}.`);
        user = await sec.updateUserInstance(user.userSystemId, urlMin.instance.instanceId);
      }

      let activeMin;

      // Processes group behaviour.

      text = text.replace(/\@\d+ /gi, '');

      let group;
      if (provider === 'chatapi') {
        // Ensures that the bot group is the active bot for the user (like switching).

        const message = req.body.messages[0];
        if (message.chatName.charAt(0) !== '+') {
          group = message.chatName;
        }
      } else if (provider === 'GeneralBots') {
        // Ensures that the bot group is the active bot for the user (like switching).

        const message = req;
        if (message.from.endsWith('@g.us')) {
          group = message.from;
        }
      }

      if (group) {
        GBLog.info(`Group: ${group}`);
        function getKeyByValue(object, value) {
          return Object.keys(object).find(key => object[key] === value);
        }
        const botId = getKeyByValue(WhatsappDirectLine.botGroups, group);
        if ((botId && user.instanceId !== this.min.instance.instanceId) || !user) {
          user = await sec.ensureUser(this.min.instance.instanceId, id, senderName, '', 'whatsApp', senderName, null);
        }
        if (botId) {
          activeMin = GBServer.globals.minInstances.filter(p => p.instance.botId === botId)[0];
          await (activeMin as any).whatsAppDirectLine.received(req, res);
          return; // EXIT HERE.
        } else {
          GBLog.warn(`Group: ${group} not associated with botId:${botId}.`);
        }
      }

      // Detects if the welcome message is enabled.

      if (process.env.WHATSAPP_WELCOME_DISABLED === 'true') {
        let minInstance = GBServer.globals.minInstances.filter(
          p => p.instance.botId.toLowerCase() === botId.toLowerCase()
        )[0];

        // Just pass the message to the receiver.

        await minInstance.whatsAppDirectLine.received(req, res);

        return;
      }

      if (!toSwitchMin) {
        toSwitchMin = GBServer.globals.minInstances.filter(p =>
          p.instance.activationCode ? p.instance.activationCode.toLowerCase() === text.toLowerCase() : false
        )[0];
      }

      // If bot has a fixed Find active bot instance.

      activeMin = botNumber ? urlMin : toSwitchMin ? toSwitchMin : GBServer.globals.minBoot;

      // If it is the first time for the user, tries to auto-execute
      // start dialog if any is specified in Config.xlsx.

      if (user === null || user.hearOnDialog) {
        user = await sec.ensureUser(activeMin.instance.instanceId, id, senderName, '', 'whatsapp', senderName, null);

        const startDialog = user.hearOnDialog
          ? user.hearOnDialog
          : activeMin.core.getParam(activeMin.instance, 'Start Dialog', null);

        if (startDialog) {
          GBLog.info(`Calling /start to Auto start ${startDialog} for ${activeMin.instance.instanceId}...`);
          if (provider === 'chatapi') {
            req.body.messages[0].body = `/start`;
          } else if (provider === 'maytapi') {
            req.body.message = `/start`;
          } else {
            req.body = `/start`;
          }

          // Resets HEAR ON DIALOG value to none and passes
          // current dialog to the direct line.

          await sec.updateUserHearOnDialog(user.userId, null);
          await (activeMin as any).whatsAppDirectLine.received(req, res);
        } else {
          await (activeMin as any).whatsAppDirectLine.sendToDevice(
            id,
            `Olá! Seja bem-vinda(o)!\nMe chamo ${activeMin.instance.title}. Como posso ajudar? Pode me falar que eu te ouço, me manda um aúdio.`,
            null
          );
          if (res) {
            res.end();
          }
        }
      } else {
        // User wants to switch bots.

        if (toSwitchMin !== undefined) {
          GBLog.info(`Switching bots from ${botId} to ${toSwitchMin.botId}...`);

          // So gets the new bot instance information and prepares to
          // auto start dialog if any is specified.

          activeMin = toSwitchMin;
          const instance = await this.min.core.loadInstanceByBotId(activeMin.botId);
          await sec.updateUserInstance(id, instance.instanceId);
          await (activeMin as any).whatsAppDirectLine.resetConversationId(activeMin.botId, id, '');
          const startDialog = activeMin.core.getParam(activeMin.instance, 'Start Dialog', null);

          if (startDialog) {

            GBLog.info(`Calling /start for Auto start : ${startDialog} for ${activeMin.instance.botId}...`);
            if (provider === 'chatapi') {
              req.body.messages[0].body = `/start`;
            } else if (provider === 'maytapi') {
              req.body.message = `/start`;
            } else {
              req.body = `/start`;
            }

            await (activeMin as any).whatsAppDirectLine.received(req, res);
          } else {
            await (activeMin as any).whatsAppDirectLine.sendToDevice(
              id,
              `Agora falando com ${activeMin.instance.title}...`,
              null
            );
          }
          if (res) {
            res.end();
          }
        } else {
          let t;
          activeMin = GBServer.globals.minInstances.filter(p => p.instance.instanceId === user.instanceId)[0];
          if (activeMin === undefined) {
            activeMin = GBServer.globals.minBoot;
            t = (activeMin as any).whatsAppDirectLine;
            await t.sendToDevice(
              id,
              `O outro Bot que você estava falando(${user.instanceId}), não está mais disponível. Agora você está falando comigo, ${activeMin.instance.title}...`
            );
          } else {
            if ((activeMin as any).whatsAppDirectLine) {
              t = (activeMin as any).whatsAppDirectLine;
            } else {
              t = (GBServer.globals.minBoot as any).whatsAppDirectLine;
            }
          }

          t.received(req, res);
        }
      }
    } catch (error) {
      error = error['e'] ? error['e'] : error;
      GBLog.error(`Error on Whatsapp callback: ${error.data ? error.data : error} ${error.stack}`);
    }
  }
}
