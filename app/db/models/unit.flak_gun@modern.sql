-- unit.flak_gun@modern: Flak gun (mesh.flak_gun_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.flak_gun_modern','Flak gun');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.flak_gun_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('loader','figure','mat.cloth',1,'','{"x":0.95,"z":-0.45,"yaw":-40,"attack":7,"hold_right":50,"hold_left":50,"legs":"#4d4c3a","far":0}'),
  ('loader_helmet','dome','mat.olive_drab',0,'loader.head','{"y":-0.13,"w":0.32,"h":0.17,"segments":6,"far":0}'),
  ('wheels','wheel','mat.rubber',0,'','{"x":-0.52,"z":0.42,"w":0.46,"t":0.14,"spokes":1,"segments":8,"repeat":2,"dx":1.04}'),
  ('wheels_back','wheel','mat.rubber',0,'','{"x":-0.52,"z":-0.48,"w":0.46,"t":0.14,"spokes":1,"segments":8,"repeat":2,"dx":1.04}'),
  ('platform','box','mat.olive_drab',0,'','{"y":0.26,"w":0.84,"d":1.4,"h":0.1}'),
  ('outriggers','box','mat.olive_drab',0,'','{"y":0.08,"w":1.9,"d":0.1,"h":0.06}'),
  ('mount','cylinder','mat.olive_drab',1,'','{"y":0.36,"w":0.72,"h":0.12,"segments":10,"traverse":35}'),
  ('cradle','box','mat.olive_drab',0,'mount','{"y":0.12,"w":0.46,"d":0.5,"h":0.3}'),
  ('seat','box','mat.khaki',0,'mount','{"x":-0.26,"y":0.12,"z":-0.24,"w":0.18,"d":0.18,"h":0.08,"far":0}'),
  ('shield','box','mat.olive_drab',1,'mount','{"y":0.12,"z":0.3,"w":0.7,"d":0.03,"h":0.34,"taper":0.15}'),
  ('barrels','cylinder','mat.cast_iron',0,'cradle','{"x":-0.1,"y":0.18,"z":0.05,"w":0.06,"h":1.15,"segments":6,"pitch":24,"recoil":0.06,"repeat":2,"dx":0.2}'),
  ('breeches','box','mat.cast_iron',0,'cradle','{"x":-0.1,"y":0.22,"z":-0.12,"w":0.1,"d":0.22,"h":0.2,"repeat":2,"dx":0.2,"far":0}'),
  ('gunner','figure','mat.cloth',1,'mount','{"x":-0.26,"y":-0.2,"z":-0.24,"seat":1,"attack":7,"hold_right":60,"hold_left":60,"legs":"#4d4c3a","far":0}'),
  ('gunner_helmet','dome','mat.olive_drab',0,'gunner.head','{"y":-0.13,"w":0.32,"h":0.17,"segments":6,"far":0}'),
  ('spark','cone','mat.flash',0,'barrels','{"y":1.15,"w":0.1,"h":0.1,"segments":3,"flash":4,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.34,"w":0.32,"h":0.34,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.flak_gun_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.flak_gun'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.flak_gun_modern'), (SELECT id FROM def WHERE key='mat.olive_drab'), 100);
