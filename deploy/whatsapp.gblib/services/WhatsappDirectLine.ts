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
| but WITHOUT ANY WARRANTY; without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of Pragmatismo.io.                 |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

const Path = require("path");
const Fs = require("fs");
const _ = require("lodash");
const Parse = require("csv-parse");
const Async = require("async");
const UrlJoin = require("url-join");
const Walk = require("fs-walk");
const logger = require("../../../src/logger");
const Swagger = require('swagger-client');
const rp = require('request-promise');
import * as request from "request-promise-native";

import { GBServiceCallback, GBService, IGBInstance } from "botlib";

export class WhatsappDirectLine extends GBService {

    pollInterval = 1000;
    directLineClientName = 'DirectLineClient';
    directLineSpecUrl = 'https://docs.botframework.com/en-us/restapi/directline3/swagger.json';

    directLineClient: any;
    whatsappServiceKey: string;
    whatsappServiceNumber: string;
    whatsappServiceUrl: string;
    whatsappServiceWebhookUrl: string;
    botId: string;
    watermark: string = null;

    conversationIds = {};

    constructor(botId, directLineSecret, whatsappServiceKey, whatsappServiceNumber, whatsappServiceUrl, whatsappServiceWebhookUrl) {

        super();

        this.botId = botId;
        this.whatsappServiceKey = whatsappServiceKey;
        this.whatsappServiceNumber = whatsappServiceNumber;
        this.whatsappServiceUrl = whatsappServiceUrl;
        this.whatsappServiceWebhookUrl = whatsappServiceWebhookUrl;

        // TODO: Migrate to Swagger 3.
        this.directLineClient = rp(this.directLineSpecUrl)
            .then((spec) => {
                return new Swagger({
                    spec: JSON.parse(spec.trim()),
                    usePromise: true
                });
            })
            .then(async (client) => {
                client.clientAuthorizations.add('AuthorizationBotConnector',
                    new Swagger.ApiKeyAuthorization('Authorization', 'Bearer ' +
                        directLineSecret, 'header'));

                var options = {
                    method: 'POST',
                    url: UrlJoin(this.whatsappServiceUrl, "webhook"),
                    qs:
                        {
                            token: this.whatsappServiceKey,
                            webhookUrl: `${this.whatsappServiceWebhookUrl}/instances/${this.botId}/whatsapp`,
                            set: true
                        },
                    headers:
                        {
                            'cache-control': 'no-cache'
                        }
                };

                try {
                    const result = await request.post(options);
                    logger.info(result);
                } catch (error) {
                    logger.error('Error initializing 3rd party Whatsapp provider.', error);
                }

                return client;
            })
            .catch((err) => {
                logger.error('Error initializing DirectLine client', err);
            });

    }

    received(req, res) {
        let text = req.body.messages[0].body;
        let from = req.body.messages[0].author.split('@')[0];
        let fromName = req.body.messages[0].senderName;

        if (req.body.messages[0].fromMe) {
            return; // Exit here.
        }

        logger.info(`GBWhatsapp: Hook called. from: ${from}(${fromName}), text: ${text})`);

        let conversationId = this.conversationIds[from];

        this.directLineClient.then((client) => {

            if (this.conversationIds[from] == null) {

                logger.info(`GBWhatsapp: Starting new conversation on Bot.`);
                client.Conversations.Conversations_StartConversation()
                    .then((response) => {
                        return response.obj.conversationId;
                    })
                    .then((conversationId) => {

                        this.conversationIds[from] = conversationId;

                        this.inputMessage(client, conversationId, text,
                            from, fromName);

                        this.pollMessages(client, conversationId, from, fromName);
                    })
                    .catch((err) => {
                        console.error('Error starting conversation', err);
                    });

            } else {
                this.inputMessage(client, conversationId, text,
                    from, fromName);
            }
            res.end();
        });
    }


    inputMessage(client, conversationId, text, from, fromName) {

        client.Conversations.Conversations_PostActivity(
            {
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
            }).catch((err) => {
                logger.error(`GBWhatsapp: Error receiving message: ${err}.`);
            });

    }

    pollMessages(client, conversationId, from, fromName) {

        logger.info(`GBWhatsapp: Starting polling message for conversationId: 
        ${conversationId}.`);

        setInterval(() => {
            client.Conversations.Conversations_GetActivities({
                conversationId:
                    conversationId, watermark: this.watermark
            })
                .then((response) => {
                    this.watermark = response.obj.watermark;
                    return response.obj.activities;
                })
                .then((activities) => {
                    this.printMessages(activities, conversationId, from, fromName);
                });
        }, this.pollInterval);
    }

    printMessages(activities, conversationId, from, fromName) {

        if (activities && activities.length) {

            // Ignore own messages.
// TODO: this.botId instead of "general-bot-9672a8d3"
            activities = activities.filter((m) => { return m.from.id === "general-bot-9672a8d3" && m.type === "message" });

            if (activities.length) {

                // Print other messages.

                activities.forEach(activity => {
                    this.printMessage(activity, conversationId, from, fromName);
                });
            }
        }
    }

    printMessage(activity, conversationId, from, fromName) {

        let output = "";

        if (activity.text) {
            logger.info(`GBWhatsapp: MSG: ${activity.text}`);
            output = activity.text;
        }

        if (activity.attachments) {
            activity.attachments.forEach((attachment) => {
                switch (attachment.contentType) {
                    case "application/vnd.microsoft.card.hero":
                        output += `\n${this.renderHeroCard(attachment)}`;
                        break;

                    case "image/png":
                        logger.info('Opening the requested image ' + attachment.contentUrl);
                        output += `\n${attachment.contentUrl}`;
                        break;
                }
            });
        }

        this.sendToDevice(conversationId, from, fromName, output);
    }

    renderHeroCard(attachment) {
        return `${attachment.content.title} - ${attachment.content.text}`;
    }

    async sendToDevice(conversationId, to, toName, msg) {
        var options = {
            method: 'POST',
            url: UrlJoin(this.whatsappServiceUrl, 'message'),
            qs:
                {
                    token: this.whatsappServiceKey,
                    phone: to,
                    body: msg
                },
            headers:
                {
                    'cache-control': 'no-cache'
                }
        };

        const result = await request.get(options);
    }
}