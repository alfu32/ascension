-- bld.temple@automation: Future temple (mesh.temple_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.temple_automation','Future temple');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.temple_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.white_composite',0,'','{"w":3.7,"d":3.7,"h":0.45,"steps":3,"inset":0.1}'),
  ('base_glow','box','mat.light_strip',0,'','{"y":0.45,"z":1.84,"w":3.5,"d":0.02,"h":0.02,"far":0}'),
  ('nave','box','mat.composite_panel',0,'','{"y":0.45,"z":-0.35,"yaw":90,"w":2.7,"d":2.1,"h":1.5,"gable":50,"windows":5,"window_w":0.12,"window_h":1.15,"window_y":0.2,"side_windows":0,"opening":"#2c5258"}'),
  ('nave_roof','gable_roof','mat.white_composite',0,'','{"y":1.95,"z":-0.35,"yaw":90,"w":2.7,"d":2.1,"pitch":50,"overhang":0.12,"t":0.08}'),
  ('nave_roof_cap','gable_roof','mat.white_composite',1,'','{"y":2.9684,"z":-0.35,"yaw":90,"w":2.94,"d":0.6,"pitch":50,"overhang":0,"t":0.03}'),
  ('ridge_glow','box','mat.light_strip',0,'','{"y":3.2,"z":-0.35,"w":0.04,"d":2.9,"h":0.03,"far":0}'),
  ('rose','shield','mat.smart_glass',0,'','{"y":2.25,"z":-1.71,"yaw":180,"w":0.6,"t":0.02,"boss":0,"segments":10,"rim":"#e8ecec","far":0}'),
  ('columns','column_row','mat.white_composite',0,'','{"y":0.45,"z":1.42,"count":4,"radius":0.07,"spacing":0.6,"h":1.75,"segments":8,"capital":0}'),
  ('slab','box','mat.white_composite',0,'','{"y":2.2,"z":1.2,"w":2.2,"d":0.66,"h":0.14}'),
  ('slab_glow','box','mat.light_strip',0,'','{"y":2.18,"z":1.54,"w":2.2,"d":0.02,"h":0.03,"far":0}'),
  ('pediment','box','mat.white_composite',0,'','{"y":2.34,"z":1.2,"yaw":90,"w":0.66,"d":2.2,"h":0.02,"gable":26}'),
  ('portico_roof','gable_roof','mat.white_composite',1,'','{"y":2.36,"z":1.2,"yaw":90,"w":0.66,"d":2.2,"pitch":26,"overhang":0.04,"t":0.05}'),
  ('portal','box','mat.smart_glass',0,'','{"y":0.45,"z":1,"w":0.8,"d":0.04,"h":1.3,"far":0}'),
  ('spire','cylinder','mat.white_composite',0,'','{"x":1.4,"y":0.45,"z":1.2,"w":0.4,"h":3.6,"taper":0.55,"segments":8}'),
  ('spire_band','cylinder','mat.white_composite',1,'','{"x":1.4,"y":3.2,"z":1.2,"w":0.3,"h":0.25,"segments":8}'),
  ('spire_light','cylinder','mat.light_strip',0,'','{"x":1.4,"y":4.05,"z":1.2,"w":0.04,"h":0.6,"segments":4,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.temple_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.temple'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.temple_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
