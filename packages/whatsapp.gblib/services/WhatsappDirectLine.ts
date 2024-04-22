/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.         |
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
| "General Bots" is a registered trademark of pragmatismo.cloud.             |
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
import { GBLog, GBMinInstance, GBService, IGBPackage } from 'botlib';
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
import { GBUtil } from '../../../src/util.js';
const { WAState, Client, MessageMedia } = pkg;
import twilio from 'twilio';
import { GBVMService } from '../../basic.gblib/services/GBVMService.js';
import { GBLogEx } from '../../core.gbapp/services/GBLogEx.js';


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
        ? 'GeneralBots' : 'official';
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
        req.headers['Authorization'] = `Bearer ${this.min.instance.webchatKey}`;
      }
    });
    this.directLineClient = client;

    let url: string;
    let options: any;

    switch (this.provider) {
      case 'official':
        const accountSid = process.env.TWILIO_ACCOUNT_SID;
        const authToken = process.env.TWILIO_AUTH_TOKEN;
        this.customClient = twilio(null, authToken, { accountSid: accountSid });

        break;
      case 'GeneralBots':
        const minBoot = GBServer.globals.minBoot;
        // Initialize the browser using a local profile for each bot.
        const gbaiPath = DialogKeywords.getGBAIPath(this.min.botId);
        const webVersion  = '2.2411.2';
        const localName = Path.join('work', gbaiPath, 'profile');
        const createClient = () => {
          const client = (this.customClient = new Client({
            puppeteer: GBSSR.preparePuppeteer(localName)
            , webVersionCache: { type: 'remote', 
            remotePath: `https://raw.githubusercontent.com/wppconnect-team/wa-version/main/html/${webVersion}.html` } 
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
              const pid = GBVMService.createProcessInfo(null, this.min, 'wppboot', null);
              
              // Sends QR Code to boot bot admin.
              
              const msg = `Please, scan QR Code with for bot ${this.botId}.`;
              qrcode.generate(qr, { small: true, scale: 0.5 });

              const s = new DialogKeywords();
              const qrBuf = await s.getQRCode({pid, text: qr});
              const localName = Path.join('work', gbaiPath, 'cache', `qr${GBAdminService.getRndReadableIdentifier()}.png`);
              Fs.writeFileSync(localName, qrBuf.data);
              const url = urlJoin(
                GBServer.globals.publicAddress,
                this.min.botId,
                'cache',
                Path.basename(localName)
              );
              
              if (adminNumber){
                await GBServer.globals.minBoot.whatsAppDirectLine.sendFileToDevice(adminNumber, url, Path.basename(localName), msg);
              }
                            
              if (adminEmail){
                await s.sendEmail({pid, to: adminEmail, subject: `Check your WhatsApp for bot ${this.min.botId}`,
                  body: msg
                });
              }

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
              const wait = Math.floor(Math.random() * 5000) + 1000;
              await GBUtil.sleep(wait);
              if (chat.isGroup) {
                // await chat.clearMessages();
              } else if (!chat.pinned) {
                await chat.delete();
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
    }

    if (setUrl && options && this.whatsappServiceUrl) {
      GBServer.globals.server.use(`/audios`, express.static('work'));

      if (options) {
        try {
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
    return req.body.ProfileName ? 'official' : 'GeneralBots';
  }

  public async received(req, res) {
    const provider = WhatsappDirectLine.providerFromRequest(req);

    let message, to, from, fromName, text: string;
    let group = '';
    let answerText = null;
    let attachments = null;

    switch (provider) {

      case 'official':
        message = req.body;
        from = req.body.From.replace(/whatsapp\:\+/gi, '');
        to = req.body.To.replace(/whatsapp\:\+/gi, '');
        text = req.body.Body;
        fromName = req.body.ProfileName;
        break;
      case 'GeneralBots':
        message = req;
        to = message.to.endsWith('@g.us') ? message.to.split('@')[0] : message.to.split('@')[0];
        const newThis = WhatsappDirectLine.botsByNumber[to];

        // If there is a number specified, checks if it
        // is related to a custom bot and reroutes immediately.

        if (newThis && newThis !== this && newThis.min.botId !== GBServer.globals.minBoot.botId) {
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
    }

    text = text.replace(/\@\d+ /gi, '');
    GBLogEx.info(0, `GBWhatsapp: RCV ${from}(${fromName}): ${text})`);

    let botGroupID = WhatsappDirectLine.botGroups[this.min.botId];
    let botShortcuts = this.min.core.getParam<string>(this.min.instance, 'WhatsApp Group Shortcuts', null);
    if (!botShortcuts) {
      botShortcuts = new Array();
    } else {
      botShortcuts = botShortcuts.split(' ');
    }

    if (provider === 'GeneralBots') {
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
        });

        // Ignore group messages without the mention to Bot.

        let botNumber = this.min.core.getParam<string>(this.min.instance, 'Bot Number', null);
        if (botNumber && !answerText && !found) {
          botNumber = botNumber.replace('+', '');
          if (!message.body.startsWith('@' + botNumber)) {

            return;
          }
        }
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
    const user = await sec.ensureUser(this.min, from, fromName, '', 'whatsapp', fromName, null);
    const locale = user.locale ? user.locale : 'pt';

    if (answerText) {
      await this.sendToDeviceEx(user.userSystemId, answerText, locale, null);

      return; // Exit here.
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
          GBLogEx.info(this.min, `HUMAN AGENT (${manualUser.agentSystemId}) TO USER ${manualUser.userSystemId}: ${text}`);
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
        GBLogEx.info(this.min, `USER (${from}) TO AGENT ${agent.userSystemId}: ${text}`);

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
        GBLogEx.info(this.min, `GBWhatsapp: Starting new conversation on Bot.`);
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
    GBLogEx.info(this.min, `GBWhatsapp: Starting message polling(${from}, ${conversationId}).`);

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
          `Error calling printMessages on Whatsapp channel ${err.data === undefined ? err : err.data} ${err.errObj ? err.errObj.message : ''
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
      GBLogEx.info(this.min, `GBWhatsapp: SND ${to}(${toName}): ${activity.text}`);
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
            GBLogEx.info(this.min, `Unknown content type: ${attachment.contentType}`);
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

    }

    if (options) {
      try {
        // tslint:disable-next-line: await-promise
        const result = await fetch(url, options);
        GBLogEx.info(this.min, `File ${url} sent to ${to}: ${result}`);
      } catch (error) {
        GBLog.error(`Error sending file to Whatsapp provider ${error.message}`);
      }
    }
  }

  public async sendAudioToDevice(to, url) {
    let options;
    switch (this.provider) {
      case 'GeneralBots':
        const attachment = MessageMedia.fromUrl(url);
        await this.customClient.sendMessage(to, attachment);

        break;

    }

    if (options) {
      try {
        const result = await fetch(url, options);
        GBLogEx.info(this.min, `Audio ${url} sent to ${to}: ${result}`);
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

        case 'official':
          const botNumber = this.min.core.getParam(this.min.instance, 'Bot Number', null);
          if (to.charAt(0) !== '+') {
            to = `+${to}`
          }

          let messages = msg.match(/(.|[\r\n]){1,1000}/g)

          await CollectionUtil.asyncForEach(messages, async msg => {
            await GBUtil.sleep(3000);
            await this.customClient.messages
              .create({
                body: msg,
                from: `whatsapp:${botNumber}`,
                to: `whatsapp:${to}`
                // TODO: mediaUrl.
              });

          });
        
        break;

        case 'GeneralBots':
          to = to.replace('+', '');
          if (to.indexOf('@') == -1) {
            if (to.length == 18) {
              to = to + '@g.us';
            } else {
              to = to + '@c.us';
            }
          }
          if (await this.customClient.getState() === WAState.CONNECTED) {
            await this.customClient.sendMessage(to, msg);
          }
          else {
            GBLogEx.info(this.min, `WhatsApp OFFLINE ${to}: ${msg}`);
          }

          break;

      }

      if (options) {
        try {
          GBLogEx.info(this.min, `Message [${msg}] is being sent to ${to}...`);
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

  private async WhatsAppCallback(req, res, botId = null) {
    try {
      if (!req.body && req.type !== 'ptt') {
        return;
      }

      let provider = GBMinService.getProviderName(req, res);
      let id;
      let senderName;
      let text;

      switch (provider) {

        case 'official':

          const { body } = req;

          id = body.From.replace(/whatsapp\:\+/, '');
          senderName = body.ProfileName;
          text = body.Body;

          break;

        case 'GeneralBots':
          // Ignore E2E messages and status updates.

          if ((req.type && req.type === 'e2e_notification') || req.isStatus) {
            return;
          }

          id = req.from.split('@')[0];
          senderName = req._data.notifyName;
          text = req.body;
          botId=botId?? this.botId;
          break;

      }

      const sec = new SecService();

      // Tries to find if user wants to switch bots.

      let toSwitchMin = GBServer.globals.minInstances.filter(
        p => p.instance.botId.toLowerCase() === text.toLowerCase()
      )[0];
      

      botId = botId??GBServer.globals.minBoot.botId;
      GBLogEx.info(this.min, `A WhatsApp mobile requested instance for: ${botId}.`);
        
      let urlMin: any = GBServer.globals.minInstances.filter(p => p.instance.botId === botId)[0];
      // Detects user typed language and updates their locale profile if applies.
      let min = urlMin;

      let user = await sec.getUserFromSystemId(id);

      const botNumber = urlMin ? urlMin.core.getParam(urlMin.instance, 'Bot Number', null) : null;
      if (botNumber && GBServer.globals.minBoot.botId !== urlMin.botId) {
        GBLogEx.info(this.min, `${id} fixed by bot number talked to: ${botId}.`);
        let locale = user?.locale ? user.locale : min.core.getParam(
          min.instance,
          'Default User Language',
          GBConfigService.get('DEFAULT_USER_LANGUAGE'));
        ;

        if (!user) {


          const detectLanguage =
            min.core.getParam(
              min.instance,
              'Language Detector',
              false) != false;


          if (text != '' && detectLanguage) {
            locale = await min.conversationalService.getLanguage(min, text);
            GBLogEx.info(this.min, `${locale} defined for first time mobile: ${id}.`);
          }
        }

        user = await sec.ensureUser(urlMin, id, '', '', 'omnichannel', '', '');
        user = await sec.updateUserInstance(id, urlMin.instance.instanceId);
        if (locale) {
          user = await sec.updateUserLocale(user.userId, locale);
        }
      }
      if (req.type === 'ptt') {
        if (process.env.AUDIO_DISABLED !== 'true') {
          const media = await req.downloadMedia();
          const buf = Buffer.from(media.data, 'base64');

          text = await GBConversationalService.getTextFromAudioBuffer(
            this.min.instance.speechKey,
            this.min.instance.cloudLocation,
            buf,
            user.locale
          );

          req.body = text;

        } else {
          await this.sendToDevice(user.userSystemId, `No momento estou apenas conseguindo ler mensagens de texto.`, null);
        }
      }



      let activeMin;

      // Processes group behaviour.

      text = text.replace(/\@\d+ /gi, '');

      let group;
      if (provider === 'GeneralBots') {
        // Ensures that the bot group is the active bot for the user (like switching).

        const message = req;
        if (message.from.endsWith('@g.us')) {
          group = message.from;
        }
      }

      if (group) {
        GBLogEx.info(this.min, `Group: ${group}`);
        function getKeyByValue(object, value) {
          return Object.keys(object).find(key => object[key] === value);
        }
        const botId = getKeyByValue(WhatsappDirectLine.botGroups, group);
        if ((botId && user.instanceId !== this.min.instance.instanceId) || !user) {
          user = await sec.ensureUser(this.min, id, senderName, '', 'whatsApp', senderName, null);
        }
        if (botId) {
          activeMin = GBServer.globals.minInstances.filter(p => p.instance.botId === botId)[0];
          await (activeMin as any).whatsAppDirectLine.received(req, res);
          return; // EXIT HERE.
        } else {
          GBLog.warn(`Group: ${group} not associated with botId:${botId}.`);
          return;
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
      min = activeMin;
      // If it is the first time for the user, tries to auto-execute
      // start dialog if any is specified in Config.xlsx.

      if (user === null || user.hearOnDialog) {

        user = await sec.ensureUser(activeMin, id, senderName, '', 'whatsapp', senderName, null);

        const startDialog = user.hearOnDialog
          ? user.hearOnDialog
          : activeMin.core.getParam(activeMin.instance, 'Start Dialog', null);

        if (startDialog) {
          GBLogEx.info(this.min, `Calling /start to Auto start ${startDialog} for ${activeMin.instance.instanceId}...`);
          if (provider === 'GeneralBots') {
            req.body = `/start`;
          }

          // Resets HEAR ON DIALOG value to none and passes
          // current dialog to the direct line.

          await sec.updateUserHearOnDialog(user.userId, null);
          await (activeMin as any).whatsAppDirectLine.received(req, res);
        } else {
          if (res) {
            res.end();
          }
        }
      } else {
        // User wants to switch bots.

        if (toSwitchMin) {
          GBLogEx.info(this.min, `Switching bots from ${botId} to ${toSwitchMin.botId}...`);

          // So gets the new bot instance information and prepares to
          // auto start dialog if any is specified.

          activeMin = toSwitchMin;
          const instance = await this.min.core.loadInstanceByBotId(activeMin.botId);
          user = await sec.updateUserInstance(id, instance.instanceId);
          await (activeMin as any).whatsAppDirectLine.resetConversationId(activeMin.botId, id, '');
          const startDialog = activeMin.core.getParam(activeMin.instance, 'Start Dialog', null);

          if (startDialog) {

            GBLogEx.info(this.min, `Calling /start for Auto start : ${startDialog} for ${activeMin.instance.botId}...`);
            if (provider === 'GeneralBots') {
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

          await t.received(req, res);
        }
      }
    } catch (error) {
      error = error['e'] ? error['e'] : error;
      GBLog.error(`Error on Whatsapp callback: ${error.data ? error.data : error} ${error.stack}`);
    }
  }
}
