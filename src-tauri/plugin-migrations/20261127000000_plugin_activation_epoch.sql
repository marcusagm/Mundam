ALTER TABLE plugin_catalog_activations ADD COLUMN activation_epoch TEXT NOT NULL DEFAULT '';

UPDATE plugin_catalog_activations SET activation_epoch = lower(hex(randomblob(16)));
