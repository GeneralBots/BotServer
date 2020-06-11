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

import urlJoin = require('url-join');

const Swagger = require('swagger-client');
const rp = require('request-promise');
const fs = require('fs');
import { GBLog, GBService, GBMinInstance } from 'botlib';
import * as request from 'request-promise-native';
import { GBServer } from '../../../src/app';
import { GBConversationalService } from '../../core.gbapp/services/GBConversationalService';
import { SecService } from '../../security.gblib/services/SecService';
import { Messages } from '../strings';

/**
 * Support for Whatsapp.
 */
export class WhatsappDirectLine extends GBService {
  public pollInterval = 5000;
  public directLineClientName = 'DirectLineClient';

  public directLineClient: any;
  public whatsappServiceKey: string;
  public whatsappServiceNumber: string;
  public whatsappServiceUrl: string;
  public botId: string;
  private directLineSecret: string;
  private locale: string = 'pt-BR';

  public conversationIds = {};
  min: GBMinInstance;

  constructor(
    min: GBMinInstance,
    botId,
    directLineSecret,
    whatsappServiceKey,
    whatsappServiceNumber,
    whatsappServiceUrl
  ) {
    super();

    this.min = min;
    this.botId = botId;
    this.directLineSecret = directLineSecret;
    this.whatsappServiceKey = whatsappServiceKey;
    this.whatsappServiceNumber = whatsappServiceNumber;
    this.whatsappServiceUrl = whatsappServiceUrl;

  }

  public async setup(setUrl) {
    this.directLineClient =
      new Swagger({
        spec: JSON.parse(fs.readFileSync('directline-3.0.json', 'utf8')),
        usePromise: true
      });
    let client = await this.directLineClient;

    client.clientAuthorizations.add(
      'AuthorizationBotConnector',
      new Swagger.ApiKeyAuthorization('Authorization', `Bearer ${this.directLineSecret}`, 'header')
    );

    const options = {
      method: 'POST',
      url: urlJoin(this.whatsappServiceUrl, 'webhook'),
      timeout: 10000,
      qs: {
        token: this.whatsappServiceKey,
        webhookUrl: `${GBServer.globals.publicAddress}/webhooks/whatsapp`,
        set: true
      },
      headers: {
        'cache-control': 'no-cache'
      }
    };

    if (setUrl) {
      const express = require('express');
      GBServer.globals.server.use(`/audios`, express.static('work'));

      if (process.env.ENDPOINT_UPDATE === "true") {
        try {
          let res = await request.post(options);
          GBLog.info(res);
        } catch (error) {
          GBLog.error(`Error initializing 3rd party Whatsapp provider(1) ${error.message}`);
        }
      }
    }

  }

  public static async asyncForEach(array, callback) {
    for (let index = 0; index < array.length; index++) {
      await callback(array[index], index, array);
    }
  }

  public resetConversationId(number) {
    this.conversationIds[number] = undefined;
  }

  public async check() {

    GBLog.info(`GBWhatsapp: Checking server...`);

    const options = {
      url: urlJoin(this.whatsappServiceUrl, 'status') + `?token=${this.min.instance.whatsappServiceKey}`,
      method: 'GET',

    };

    const res = await request(options);
    const json = JSON.parse(res);
    return json.accountStatus === "authenticated";

  }

  public async received(req, res) {

    if (req.body.messages === undefined) {
      res.end();
      return;  // Exit here.
    }

    const message = req.body.messages[0];
    let text = message.body;
    const from = message.author.split('@')[0];
    const fromName = message.senderName;

    if (req.body.messages[0].fromMe) {
      res.end();
      return; // Exit here.
    }
    GBLog.info(`GBWhatsapp: RCV ${from}(${fromName}): ${text})`);

    const id = req.body.messages[0].chatId.split('@')[0];
    const senderName = req.body.messages[0].senderName;
    let sec = new SecService();

    const user = await sec.ensureUser(this.min.instance.instanceId, id,
      senderName, "", "whatsapp", senderName);

    const locale = user.locale ? user.locale : 'pt';
    if (message.type === "ptt") {

      if (process.env.AUDIO_DISABLED !== "true") {
        const options = {
          url: message.body,
          method: 'GET',
          encoding: 'binary'
        };

        const res = await request(options);
        let buf = Buffer.from(res, 'binary');
        text = await GBConversationalService.getTextFromAudioBuffer(
          this.min.instance.speechKey,
          this.min.instance.cloudLocation,
          buf, locale
        );
      }
      else {
        await this.sendToDevice(user.userSystemId, `No momento estou apenas conseguindo ler mensagens de texto.`);
      }
    }

    const conversationId = this.conversationIds[from];

    let client = await this.directLineClient;
    if (user.agentMode === "self") {
      let manualUser = await sec.getUserFromAgentSystemId(id);

      if (manualUser === null) {
        await sec.updateCurrentAgent(id, this.min.instance.instanceId, null);
      }
      else {
        const cmd = '/reply ';
        if (text.startsWith(cmd)) {
          let filename = text.substr(cmd.length);
          let message = await this.min.kbService.getAnswerTextByMediaName(this.min.instance.instanceId, filename);

          if (message === null) {
            await this.sendToDeviceEx(user.userSystemId, `File ${filename} not found in any .gbkb published. Check the name or publish again the associated .gbkb.`,
              locale);
          } else {
            await this.min.conversationalService.sendMarkdownToMobile(this.min, null, user.userSystemId, message);
          }
        } else if (text === '/qt') {
          // TODO: Transfers only in pt-br for now.
          await this.sendToDeviceEx(manualUser.userSystemId, Messages[this.locale].notify_end_transfer(this.min.instance.botId), locale);
          await this.sendToDeviceEx(user.agentSystemId, Messages[this.locale].notify_end_transfer(this.min.instance.botId), locale);

          await sec.updateCurrentAgent(manualUser.userSystemId, this.min.instance.instanceId, null);
        }
        else {
          GBLog.info(`HUMAN AGENT (${id}) TO USER ${manualUser.userSystemId}: ${text}`);
          this.sendToDeviceEx(manualUser.userSystemId, `${manualUser.agentSystemId}: ${text}`, locale);
        }
      }
    }
    else if (user.agentMode === "human") {
      let agent = await sec.getUserFromSystemId(user.agentSystemId);
      if (text === '/t') {
        await this.sendToDeviceEx(user.userSystemId, `Você já está sendo atendido por ${agent.userSystemId}.`, locale);
      }
      else if (text === '/qt' || text === "Sair" || text === "Fechar") {
        // TODO: Transfers only in pt-br for now.
        await this.sendToDeviceEx(id, Messages[this.locale].notify_end_transfer(this.min.instance.botId), locale);
        await this.sendToDeviceEx(user.agentSystemId, Messages[this.locale].notify_end_transfer(this.min.instance.botId), locale);

        await sec.updateCurrentAgent(id, this.min.instance.instanceId, null);
      }
      else {
        GBLog.info(`USER (${id}) TO AGENT ${user.userSystemId}: ${text}`);
        this.sendToDeviceEx(user.agentSystemId, `Bot: ${this.min.instance.botId}\n${id}: ${text}`, locale);
      }

    }
    else if (user.agentMode === "bot" || user.agentMode === null) {

      if (this.conversationIds[from] === undefined) {
        GBLog.info(`GBWhatsapp: Starting new conversation on Bot.`);
        const response = await client.Conversations.Conversations_StartConversation()
        const generatedConversationId = response.obj.conversationId;

        this.conversationIds[from] = generatedConversationId;

        this.pollMessages(client, generatedConversationId, from, fromName);
        this.inputMessage(client, generatedConversationId, text, from, fromName);
      } else {

        this.inputMessage(client, conversationId, text, from, fromName);
      }
    }
    else {
      GBLog.warn(`Inconsistencty found: Invalid agentMode on User Table: ${user.agentMode}`);
    }

    res.end();

  }

  public inputMessage(client, conversationId, text, from, fromName) {
    return client.Conversations.Conversations_PostActivity({
      conversationId: conversationId,
      activity: {
        textFormat: 'plain',
        text: text,
        type: 'message',
        from: {
          id: from,
          name: fromName
        },
        replyToId: from
      }
    });
  }

  public pollMessages(client, conversationId, from, fromName) {
    GBLog.info(`GBWhatsapp: Starting message polling(${from}, ${conversationId}).`);

    let watermark: any;

    const worker = async () => {
      try {
        const response = await client.Conversations.Conversations_GetActivities({
          conversationId: conversationId,
          watermark: watermark
        });
        watermark = response.obj.watermark;
        await this.printMessages(response.obj.activities, conversationId, from, fromName);
      } catch (err) {
        GBLog.error(`Error calling printMessages on Whatsapp channel ${err.data === undefined ? err : err.data}`);
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

  public async printMessage(activity, conversationId, from, fromName) {
    let output = '';

    if (activity.text) {
      GBLog.info(`GBWhatsapp: SND ${from}(${fromName}): ${activity.text}`);
      output = activity.text;
    }

    if (activity.attachments) {
      activity.attachments.forEach(attachment => {
        switch (attachment.contentType) {
          case 'application/vnd.microsoft.card.hero':
            output += `\n${this.renderHeroCard(attachment)}`;
            break;

          case 'image/png':
            GBLog.info(`Opening the requested image ${attachment.contentUrl}`);
            output += `\n${attachment.contentUrl}`;
            break;
          default:
            GBLog.info(`Unknown content type: ${attachment.contentType}`);
        }
      });
    }

    await this.sendToDevice(from, output);
  }

  public renderHeroCard(attachment) {
    return `${attachment.content.title} - ${attachment.content.text}`;
  }

  public async sendFileToDevice(to, url, filename, caption) {
    const options = {
      method: 'POST',
      url: urlJoin(this.whatsappServiceUrl, 'sendFile'),
      qs: {
        token: this.whatsappServiceKey,
        phone: to,
        body: url,
        filename: filename,
        caption: caption
      },
      headers: {
        'cache-control': 'no-cache'
      }
    };

    try {
      // tslint:disable-next-line: await-promise
      const result = await request.post(options);
      GBLog.info(`File ${url} sent to ${to}: ${result}`);
    } catch (error) {
      GBLog.error(`Error sending file to Whatsapp provider ${error.message}`);
    }
  }

  public async sendAudioToDevice(to, url) {
    const options = {
      method: 'POST',
      url: urlJoin(this.whatsappServiceUrl, 'sendPTT'),
      qs: {
        token: this.whatsappServiceKey,
        phone: to,
        body: url
      },
      headers: {
        'cache-control': 'no-cache'
      }
    };

    try {
      // tslint:disable-next-line: await-promise
      const result = await request.post(options);
      GBLog.info(`Audio ${url} sent to ${to}: ${result}`);
    } catch (error) {
      GBLog.error(`Error sending audio message to Whatsapp provider ${error.message}`);
    }
  }

  public async sendTextAsAudioToDevice(to, msg) {

    let url = await GBConversationalService.getAudioBufferFromText(
      this.min.instance.speechKey,
      this.min.instance.cloudLocation,
      msg, this.locale
    );

    await this.sendFileToDevice(to, url, 'Audio', msg);
  }

  public async sendToDeviceEx(to, msg, locale) {
    const minBoot = GBServer.globals.minBoot as any;

    const text = await this.min.conversationalService.translate(this.min,
      this.min.instance.translatorKey ? this.min.instance.translatorKey : minBoot.instance.translatorKey,
      this.min.instance.translatorEndpoint ? this.min.instance.translatorEndpoint : minBoot.instance.translatorEndpoint,
      msg,
      locale
    );
    await this.sendToDevice(to, text);

  }

  public async sendToDevice(to: string, msg: string) {

    const cmd = '/audio ';
    if (msg.startsWith(cmd)) {
      msg = msg.substr(cmd.length);

      return await this.sendTextAsAudioToDevice(to, msg);
    } else {

      const options = {
        method: 'POST',
        url: urlJoin(this.whatsappServiceUrl, 'message'),
        qs: {
          token: this.whatsappServiceKey,
          phone: to,
          body: msg
        },
        headers: {
          'cache-control': 'no-cache'
        }
      };

      try {
        // tslint:disable-next-line: await-promise
        const result = await request.post(options);
        GBLog.info(`Message [${msg}] sent to ${to}: ${result}`);
      } catch (error) {
        GBLog.error(`Error sending message to Whatsapp provider ${error.message}`);

        // TODO: Handle Error: socket hang up and retry.
      }
    }
  }
}
