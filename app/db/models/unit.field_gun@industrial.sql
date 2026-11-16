-- unit.field_gun@industrial: Field gun (mesh.field_gun_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.field_gun_industrial','Field gun');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.field_gun_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('gunner','figure','mat.cloth',1,'','{"x":-0.8,"z":-0.8,"yaw":15,"attack":7,"hold_right":30,"hold_left":10,"legs":"#4e4a3a","far":0}'),
  ('gunner_cap','cylinder','mat.felt',0,'gunner.head','{"y":-0.06,"w":0.23,"h":0.1,"segments":7,"far":0}'),
  ('loader','figure','mat.cloth',1,'','{"x":0.8,"z":-1,"yaw":-25,"attack":7,"hold_right":50,"hold_left":50,"legs":"#4e4a3a","far":0}'),
  ('loader_cap','cylinder','mat.felt',0,'loader.head','{"y":-0.06,"w":0.23,"h":0.1,"segments":7,"far":0}'),
  ('shell','cylinder','mat.brass',0,'loader.right_hand','{"x":0.1,"y":-0.02,"w":0.08,"h":0.3,"segments":5,"roll":90,"far":0}'),
  ('wheels','wheel','mat.oak',0,'','{"x":-0.46,"w":0.86,"t":0.08,"spokes":3,"segments":12,"repeat":2,"dx":0.92}'),
  ('axle','box','mat.cast_iron',0,'','{"y":0.39,"w":0.98,"d":0.09,"h":0.09}'),
  ('cradle','box','mat.olive_drab',0,'','{"y":0.46,"z":-0.05,"w":0.2,"d":0.8,"h":0.16}'),
  ('trail','box','mat.olive_drab',0,'','{"y":0,"z":-1.2,"w":0.18,"d":0.12,"h":1.2,"pitch":72}'),
  ('spade','box','mat.cast_iron',0,'','{"z":-1.3,"w":0.3,"d":0.05,"h":0.18,"far":0}'),
  ('shield','box','mat.olive_drab',0,'','{"y":0.48,"z":0.18,"w":1,"d":0.04,"h":0.62,"taper":0.06}'),
  ('shield_band','box','mat.cloth',1,'','{"y":0.86,"z":0.205,"w":0.92,"d":0.01,"h":0.16}'),
  ('barrel','cylinder','mat.cast_iron',0,'','{"y":0.63,"z":-0.42,"w":0.1,"h":1.55,"taper":0.3,"segments":7,"pitch":88,"recoil":0.14}'),
  ('recuperator','cylinder','mat.olive_drab',0,'barrel','{"z":0.07,"w":0.1,"h":0.75,"segments":6,"far":0}'),
  ('spark','cone','mat.flash',0,'barrel','{"y":1.55,"w":0.1,"h":0.1,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.6,"w":0.62,"h":0.6,"segments":8,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.field_gun_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.field_gun'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.field_gun_industrial'), (SELECT id FROM def WHERE key='mat.olive_drab'), 100);
