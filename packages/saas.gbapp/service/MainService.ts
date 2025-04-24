// BotServer/packages/saas.gbapp/service/MainService.ts
import { GBOnlineSubscription } from '../model/MainModel.js';
import { GBMinInstance, GBLog } from 'botlib';
import { CollectionUtil } from 'pragmatismo-io-framework';
import urlJoin from 'url-join';
import { GBOService } from './GBOService.js';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService.js';
import Stripe from 'stripe';
import { GBUtil } from '../../../src/util.js';

export class MainService {
  private gboService: GBOService;
  private stripe: Stripe;
  private readonly PAYMENT_CHECK_INTERVAL = 5000; // 5 seconds
  private readonly PAYMENT_CHECK_TIMEOUT = 300000; // 5 minutes timeout

  constructor() {
    this.gboService = new GBOService();
    this.stripe = new Stripe(process.env.STRIPE_SECRET_KEY);
  }

  public async startSubscriptionProcess(
    min: GBMinInstance,
    name: string,
    email: string,
    mobile: string,
    botName: string,
    templateName: string,
    planId: string
  ) {
    // Create initial subscription record
    const subscription = await GBOnlineSubscription.create({
      instanceId: min.instance.instanceId,
      customerName: name,
      customerEmail: email,
      customerMobile: mobile,
      botName: botName,
      planId: planId,
      status: planId === 'free' ? 'active' : 'pending_payment',
      createdAt: new Date(),
      activatedAt: planId === 'free' ? new Date() : null
    });

    if (planId === 'free') {
      return await this.createBotResources(min, subscription, templateName);
    } else {
      const priceId = this.getPriceIdForPlan(planId);
      const session = await this.stripe.checkout.sessions.create({
        payment_method_types: ['card'],
        line_items: [{
          price: priceId,
          quantity: 1,
        }],
        success_url: urlJoin(process.env.BOT_URL, min.botId, 'paymentSuccess?session_id={CHECKOUT_SESSION_ID}'),        
        mode: 'subscription',
        metadata: {
          subscriptionId: subscription.subscriptionId.toString(),
          botName: botName
        }
      });

      await subscription.update({
        stripeSessionId: session.id
      });

      return {
        paymentUrl: session.url,
        subscriptionId: subscription.subscriptionId,
        nextStep: 'Please complete the payment in the new window. I will check for completion automatically.'
      };
    }
  }

  public async waitForPaymentCompletion(
    min: GBMinInstance,
    subscriptionId: number,
    templateName: string
  ): Promise<any> {
    const startTime = Date.now();
    
    while ((Date.now() - startTime) < this.PAYMENT_CHECK_TIMEOUT) {
      const subscription = await GBOnlineSubscription.findOne({
        where: { subscriptionId }
      });

      if (!subscription) {
        throw new Error('Subscription not found');
      }

      if (subscription.stripeSessionId) {
        const session = await this.stripe.checkout.sessions.retrieve(
          subscription.stripeSessionId,
          { expand: ['payment_intent'] }
        );
        
        if (session.payment_status === 'paid') {
          await subscription.update({
            status: 'active',
            activatedAt: new Date(),
            stripePaymentIntentId: (session.payment_intent as any)?.id
          });
          
          return await this.createBotResources(min, subscription, templateName);
        }
        
        if (session.status === 'expired') {
          throw new Error('Payment failed or session expired. Please try again.');
        }
      }

      await GBUtil.sleep(this.PAYMENT_CHECK_INTERVAL);
    }

    throw new Error('Payment processing timed out. Please check your payment and try again.');
  }

  private getPriceIdForPlan(planId: string): string {
    const priceIds = {
      personal: process.env.STRIPE_PERSONAL_PLAN_PRICE_ID,
      professional: process.env.STRIPE_PROFESSIONAL_PLAN_PRICE_ID
    };

    if (!priceIds[planId]) {
      throw new Error(`No price ID configured for plan: ${planId}`);
    }

    return priceIds[planId];
  }

  private async createBotResources(
    min: GBMinInstance,
    subscription: GBOnlineSubscription,
    templateName: string
  ) {
    GBLog.info('Deploying a blank bot to storage...');
    const instance = await min.deployService.deployBlankBot(
      subscription.botName,
      subscription.customerMobile,
      subscription.customerEmail
    );

    await subscription.update({
      instanceId: instance.instanceId
    });

    let token =
      GBConfigService.get('GB_MODE') === 'legacy' ?
        await (min.adminService.acquireElevatedToken as any)(min.instance.instanceId, true) :
        null;

    let siteId = process.env.STORAGE_SITE_ID;
    let libraryId = process.env.STORAGE_LIBRARY;

    GBLog.info('Creating .gbai folder ...');
    let item = await this.gboService.createRootFolder(
      token,
      `${subscription.botName}.gbai`,
      siteId,
      libraryId
    );

    GBLog.info('Copying Templates...');
    await this.gboService.copyTemplates(min, item, templateName, 'gbkb', subscription.botName);
    await this.gboService.copyTemplates(min, item, templateName, 'gbot', subscription.botName);
    await this.gboService.copyTemplates(min, item, templateName, 'gbtheme', subscription.botName);
    await this.gboService.copyTemplates(min, item, templateName, 'gbdata', subscription.botName);
    await this.gboService.copyTemplates(min, item, templateName, 'gbdialog', subscription.botName);
    await this.gboService.copyTemplates(min, item, templateName, 'gbdrive', subscription.botName);

    GBLog.info('Configuring .gbot...');
    await min.core['setConfig'](min, instance.botId, "Can Publish", subscription.customerMobile + ";");
    await min.core['setConfig'](min, instance.botId, "Admin Notify E-mail", subscription.customerEmail);
    await min.core['setConfig'](min, instance.botId, 'WebDav Username', instance.botId);
    await min.core['setConfig'](min, instance.botId, 'WebDav Secret', instance.adminPass);

    return {
      success: true,
      botUrl: urlJoin(process.env.BOT_URL, subscription.botName)
    };
  }
}