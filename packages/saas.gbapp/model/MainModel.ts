// BotServer/packages/saas.gbapp/model/MainModel.ts
"use strict"
import { Table, Column, Model, DataType } from 'sequelize-typescript';

@Table({ tableName: 'GBOnlineSubscription' })
export class GBOnlineSubscription extends Model<GBOnlineSubscription> {
  @Column({
    primaryKey: true,
    autoIncrement: true,
    type: DataType.INTEGER
  })
  declare subscriptionId: number;

  @Column(DataType.INTEGER)
  declare instanceId: number;

  @Column(DataType.STRING(100))
  declare customerName: string;

  @Column(DataType.STRING(100))
  declare customerEmail: string;

  @Column(DataType.STRING(100))
  declare stripeSessionId: string;

  @Column(DataType.STRING(100))
  declare stripePaymentIntentId: string;
  

  @Column(DataType.STRING(20))
  declare customerMobile: string;

  @Column(DataType.STRING(50))
  declare botName: string;

  @Column(DataType.STRING(20))
  declare planId: string;

  @Column(DataType.STRING(20))
  declare status: string; // 'pending_payment', 'active', 'cancelled'

  @Column(DataType.FLOAT)
  declare paymentAmount: number;

  @Column(DataType.STRING(500))
  declare paymentUrl: string;

  @Column(DataType.STRING(100))
  declare paymentToken: string;

  @Column(DataType.STRING(4))
  declare lastCCFourDigits: string;

  @Column(DataType.DATE)
  declare createdAt: Date;

  @Column(DataType.DATE)
  declare activatedAt: Date;
}
