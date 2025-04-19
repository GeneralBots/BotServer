/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.com.br. All rights reserved.          |
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
| "General Bots" is a registered trademark of pragmatismo.com.br.              |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

'use strict';

import { GBOnlineSubscription } from '../model/MainModel.js';
import { GBMinInstance, GBLog } from 'botlib';
import { CollectionUtil } from 'pragmatismo-io-framework';
import urlJoin from 'url-join';
import { GBOService } from './GBOService.js';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService.js';
import Stripe from 'stripe';

export class MainService {
  private stripe: Stripe;

  constructor() {
    this.stripe = new Stripe(process.env.STRIPE_SECRET_KEY);
  }

  async createStripeCustomer(name: string, email: string, paymentMethodId: string) {
    const customer = await this.stripe.customers.create({
      name,
      email,
      payment_method: paymentMethodId,
      invoice_settings: {
        default_payment_method: paymentMethodId
      }
    });
    return customer;
  }

  async createStripeSubscription(customerId: string, priceId: string) {
    const subscription = await this.stripe.subscriptions.create({
      customer: customerId,
      items: [{ price: priceId }],
      expand: ['latest_invoice.payment_intent']
    });
    return subscription;
  }

  async createPaymentMethod(cardNumber: string, expMonth: number, expYear: number, cvc: string) {
    const paymentMethod = await this.stripe.paymentMethods.create({
      type: 'card',
      card: {
        number: cardNumber,
        exp_month: expMonth,
        exp_year: expYear,
        cvc: cvc
      }
    }, {});
    return paymentMethod;
  }

  async createSubscription(
    min: GBMinInstance,
    name: string,
    document: string,
    email: string,
    mobile: string,
    botName: string,
    ccNumber: string,
    ccExpiresOnMonth: number,
    ccExpiresOnYear: number,
    ccCode: string,
    templateName: string,
    free: boolean, planId: string,
  ) {
    let externalSubscriptionId = null;

    if (!free) {
      try {
        // Create Stripe payment method
        const paymentMethod = await this.createPaymentMethod(
          ccNumber,
          ccExpiresOnMonth,
          ccExpiresOnYear,
          ccCode
        );

        // Create Stripe customer
        const customer = await this.createStripeCustomer(
          name,
          email,
          paymentMethod.id
        );

        // Determine price ID based on plan
        const priceId = planId === 'professional'
          ? process.env.STRIPE_PROFESSIONAL_PRICE_ID
          : process.env.STRIPE_PERSONAL_PRICE_ID;

        // Create subscription
        const subscription = await this.createStripeSubscription(
          customer.id,
          priceId
        );

        externalSubscriptionId = subscription.id;
      } catch (error) {
        GBLog.error(`Stripe payment failed: ${error.message}`);
        throw error;
      }
    }

    // Syncs internal subscription management
    const status = free ? 'FreeTrial' : 'Active';
    GBLog.info(`Creating subscription for ${name} (${email}, ${mobile}) with status: ${status}`);

    const quantity = 1;
    const amount = 1;

    const subscription = await GBOnlineSubscription.create({
      instanceId: min.instance.instanceId,
      isFreeTrial: free,
      planId: planId,
      quantity: quantity,
      status: status,
      amount: amount,
      lastCCFourDigits: ccNumber ? ccNumber.slice(-4) : null
    });

    // Creates a bot
    GBLog.info('Deploying a blank bot to storage...');
    const instance = await min.deployService.deployBlankBot(botName, mobile, email);

    GBLog.info('Creating subscription...');
    subscription.instanceId = instance.instanceId;
    subscription.externalSubscriptionId = externalSubscriptionId;
    await subscription.save();

    let token =
      GBConfigService.get('GB_MODE') === 'legacy' ?
        await (min.adminService.acquireElevatedToken as any)(min.instance.instanceId, true) :
        null;

    let siteId = process.env.STORAGE_SITE_ID;
    let libraryId = process.env.STORAGE_LIBRARY;
    let gboService = new GBOService();

    let sleep = ms => {
      return new Promise(resolve => {
        setTimeout(resolve, ms);
      });
    };

    GBLog.info('Creating .gbai folder ...');
    let item = await gboService.createRootFolder(token, `${botName}.gbai`, siteId, libraryId);

    GBLog.info('Copying Templates...');
    await gboService.copyTemplates(min, item, templateName, 'gbkb', botName);
    await gboService.copyTemplates(min, item, templateName, 'gbot', botName);
    await gboService.copyTemplates(min, item, templateName, 'gbtheme', botName);
    await gboService.copyTemplates(min, item, templateName, 'gbdata', botName);
    await gboService.copyTemplates(min, item, templateName, 'gbdialog', botName);
    await gboService.copyTemplates(min, item, templateName, 'gbdrive', botName);

    await sleep(10000);
    GBLog.info('Configuring .gbot...');
    await min.core['setConfig'](min, instance.botId, "Can Publish", mobile + ";");
    await min.core['setConfig'](min, instance.botId, "Admin Notify E-mail", email);
    await min.core['setConfig'](min, instance.botId, 'WebDav Username', instance.botId);
    await min.core['setConfig'](min, instance.botId, 'WebDav Secret', instance.adminPass);

    GBLog.info('Bot creation done.');
  }

  public async otherTasks(min, botName, webUrl, instance, language) {
    let message = `Seu bot ${botName} está disponível no endereço: 
<br/><a href="${urlJoin(process.env.BOT_URL, botName)}">${urlJoin(process.env.BOT_URL, botName)}</a>.
<br/>
<br/>Os pacotes do General Bots (ex: .gbkb, .gbtheme) para seu Bot devem ser editados no repositório de pacotes:
<br/>
<br/><a href="${webUrl}">${webUrl}</a>. 
<br/>
<br/> Digite /publish do seu WhatsApp para publicar os pacotes. Seu número está autorizado na pasta ${botName}.gbot/Config.xlsx
<br/>
<br/>
<br/>O arquivo .zip em anexo pode ser importado no Teams conforme instruções em:
<br/><a href="https://docs.microsoft.com/en-us/microsoftteams/platform/concepts/deploy-and-publish/apps-upload">https://docs.microsoft.com/en-us/microsoftteams/platform/concepts/deploy-and-publish/apps-upload</a>. 
<br/>
<br/>Log in to the Teams client with your Microsoft 365 account.
<br/>Select Apps and choose Upload a custom app.
<br/>Select this .zip file attached to this e-mail. An install dialog displays.
<br/>Add your Bot to Teams.
<br/>
<br/>Atenciosamente, 
<br/>General Bots Online.
<br/><a href=""></a>
<br/>
<br/>E-mail remetido por Pragmatismo. 
<br/>`;

    message = await min.conversationalService.translate(
      min,
      message,
      language
    );

    GBLog.info('Generating MS Teams manifest....');

    const appManifest = await min.deployService.getBotManifest(min.instance);

    // GBLog.info( 'Sending e-mails....');
    // const emailToken = process.env.SAAS_SENDGRID_API_KEY;
    // gboService.sendEmail(
    //   emailToken,
    //   email,
    //   `${botName}`,
    //   message,
    //   message,
    //   {
    //     content: appManifest,
    //     filename: `${min.instance.botId}-Teams.zip`,
    //     type: `application/zip`,
    //     disposition: "attachment"
    //   }
    // );

    const contacts = process.env.SECURITY_LIST.split(';');

    // TODO: await CollectionUtil.asyncForEach(contacts, async item => {
    //   await (min.whatsAppDirectLine as any)['sendToDevice'](
    //     item,
    //     `Novo bot criado agora: http://gb.pragmatismo.com.br/${botName} para *${name}* (${email}, ${mobile}). Por favor, entre em contato para que mais um bot seja configurado adequadamente. `
    //   );
    // });

    // GBLog.info( 'Sharing .gbai folder...');
    // await gboService.shareFolder(token, item.parentReference.driveId, item.id, email);

  }
}
