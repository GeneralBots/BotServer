-- `6.0.2-01-core` added 48 e-commerce/fiscal columns to `products`; the canonical
-- declaration in `botschema::billing` models only the 22-column shape created by
-- `6.0.10-01-products` (plus `branch_id`, `category_id`, `is_public`).
-- The 21 columns dropped here are named by no Rust module, no `.bas`/`.gbui`
-- template (including `botserver/work/templates`, which is generated but is the
-- bot source of truth) and no later migration, so they only inflate the table.
--
-- Columns still referenced by code or by a bot template (`tax_code`, `tax_class`,
-- `global_trade_number`, `net_weight`, `gross_weight`, `width`/`height`/`length`,
-- `brand`, `model`, `color`, `size`, `material`, `gender`, `expiration_date`,
-- `min_stock`, `max_stock`, `reorder_point`, `shipping_cost`, `external_id`,
-- `external_metadata`, `slug`, `meta_description`, `tags`) are intentionally
-- kept — `store-server.gbdialog/get-ncm.bas` and `scan-barcode.bas` write
-- `tax_code`, `tax_class`, `global_trade_number`, `net_weight` and `brand`
-- straight into `products`, so those are live even without a Rust caller.

ALTER TABLE products DROP COLUMN IF EXISTS fiscal_code;
ALTER TABLE products DROP COLUMN IF EXISTS origin_code;
ALTER TABLE products DROP COLUMN IF EXISTS tax_unit_code;
ALTER TABLE products DROP COLUMN IF EXISTS package_count;
ALTER TABLE products DROP COLUMN IF EXISTS sales_tax_code;
ALTER TABLE products DROP COLUMN IF EXISTS sales_tax_rate;
ALTER TABLE products DROP COLUMN IF EXISTS excise_tax_code;
ALTER TABLE products DROP COLUMN IF EXISTS excise_tax_rate;
ALTER TABLE products DROP COLUMN IF EXISTS vat_code;
ALTER TABLE products DROP COLUMN IF EXISTS vat_rate;
ALTER TABLE products DROP COLUMN IF EXISTS service_tax_code;
ALTER TABLE products DROP COLUMN IF EXISTS service_tax_rate;
ALTER TABLE products DROP COLUMN IF EXISTS warehouse_location;
ALTER TABLE products DROP COLUMN IF EXISTS batch_number;
ALTER TABLE products DROP COLUMN IF EXISTS manufacture_date;
ALTER TABLE products DROP COLUMN IF EXISTS sale_price;
ALTER TABLE products DROP COLUMN IF EXISTS sale_start;
ALTER TABLE products DROP COLUMN IF EXISTS sale_end;
ALTER TABLE products DROP COLUMN IF EXISTS profit_margin;
ALTER TABLE products DROP COLUMN IF EXISTS external_category_id;
ALTER TABLE products DROP COLUMN IF EXISTS meta_title;
