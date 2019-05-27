import urlJoin = require('url-join');

const Swagger = require('swagger-client');
const rp = require('request-promise');
import { GBLog, GBService } from 'botlib';
import * as request from 'request-promise-native';
import { GBServer } from '../../../src/app';

/**
 * Support for Whatsapp.
 */
export class WhatsappDirectLine extends GBService {
  public pollInterval = 1000;
  public directLineClientName = 'DirectLineClient';

  public directLineClient: any;
  public whatsappServiceKey: string;
  public whatsappServiceNumber: string;
  public whatsappServiceUrl: string;
  public whatsappServiceWebhookUrl: string;
  public botId: string;
  public watermark: string;

  public conversationIds = {};

  constructor(
    botId,
    directLineSecret,
    whatsappServiceKey,
    whatsappServiceNumber,
    whatsappServiceUrl,
    whatsappServiceWebhookUrl
  ) {
    super();

    this.botId = botId;
    this.whatsappServiceKey = whatsappServiceKey;
    this.whatsappServiceNumber = whatsappServiceNumber;
    this.whatsappServiceUrl = whatsappServiceUrl;
    this.whatsappServiceWebhookUrl = whatsappServiceWebhookUrl;
    var fs = require('fs')

    this.directLineClient =
      new Swagger({
        spec: JSON.parse(fs.readFileSync('directline-3.0.json', 'utf8')),
        usePromise: true
      });
    this.directLineClient
      .then(async client => {

        client.clientAuthorizations.add(
          'AuthorizationBotConnector',
          new Swagger.ApiKeyAuthorization('Authorization', `Bearer ${directLineSecret}`, 'header')
        );

        const options = {
          method: 'POST',
          url: urlJoin(this.whatsappServiceUrl, 'webhook'),
          qs: {
            token: this.whatsappServiceKey,
            webhookUrl: `${GBServer.globals.publicAddress}/instances/${this.botId}/whatsapp`,
            set: true
          },
          headers: {
            'cache-control': 'no-cache'
          }
        };

        try {
          const result = request.post(options);
          GBLog.info(result);
        } catch (error) {
          GBLog.error(`Error initializing 3rd party Whatsapp provider(1) ${error}`);
        }
      });
  }

  public received(req, res) {
    const text = req.body.messages[0].body;
    const from = req.body.messages[0].author.split('@')[0];
    const fromName = req.body.messages[0].senderName;

    if (req.body.messages[0].fromMe) {
      return; // Exit here.
    }

    GBLog.info(`GBWhatsapp: Hook called. from: ${from}(${fromName}), text: ${text})`);

    const conversationId = this.conversationIds[from];

    this.directLineClient.then(client => {
      if (this.conversationIds[from] === undefined) {
        GBLog.info(`GBWhatsapp: Starting new conversation on Bot.`);
        client.Conversations.Conversations_StartConversation()
          .then(response => {
            return response.obj.conversationId;
          })
          .then(generatedConversationId => {
            this.conversationIds[from] = generatedConversationId;
            this.inputMessage(client, generatedConversationId, text, from, fromName);

            this.pollMessages(client, generatedConversationId, from, fromName);
          })
          .catch(err => {
            GBLog.error(`Error starting conversation ${err}`);
          });
      } else {
        this.inputMessage(client, conversationId, text, from, fromName);
      }
      res.end();
    });
  }

  public inputMessage(client, conversationId, text, from, fromName) {
    client.Conversations.Conversations_PostActivity({
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
    }).catch(err => {
      GBLog.error(`GBWhatsapp: Error receiving message: ${err}.`);
    });
  }

  public pollMessages(client, conversationId, from, fromName) {
    GBLog.info(`GBWhatsapp: Starting polling message for conversationId:
        ${conversationId}.`);

    setInterval(() => {
      client.Conversations.Conversations_GetActivities({
        conversationId: conversationId,
        watermark: this.watermark
      })
        .then(response => {
          this.watermark = response.obj.watermark;

          return response.obj.activities;
        })
        .then(activities => {
          this.printMessages(activities, conversationId, from, fromName);
        });
    }, this.pollInterval);
  }

  public printMessages(activities, conversationId, from, fromName) {
    if (activities && activities.length) {
      // Ignore own messages.

      activities = activities.filter(m => m.from.id === this.botId && m.type === 'message');

      if (activities.length) {
        // Print other messages.

        activities.forEach(activity => {
          this.printMessage(activity, conversationId, from, fromName);
        });
      }
    }
  }

  public printMessage(activity, conversationId, from, fromName) {
    let output = '';

    if (activity.text) {
      GBLog.info(`GBWhatsapp: MSG: ${activity.text}`);
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

    this.sendToDevice(from, output);
  }

  public renderHeroCard(attachment) {
    return `${attachment.content.title} - ${attachment.content.text}`;
  }

  public async sendToDevice(to, msg) {
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
      const result = request.post(options);
      GBLog.info(result);
    } catch (error) {
      GBLog.error(`Error sending message to Whatsapp provider ${error}`);
    }
  }
}
