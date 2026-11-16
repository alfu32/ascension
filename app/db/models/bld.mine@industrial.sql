-- bld.mine@industrial: Industrial mine (mesh.mine_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.mine_industrial','Industrial mine');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.mine_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.cinders',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('collar','wall','mat.red_brick',0,'','{"x":-0.6,"z":-0.4,"w":1.1,"d":1.1,"h":0.35,"t":0.18,"ring":1}'),
  ('pit','box','mat.rough_stone',0,'','{"x":-0.6,"y":0.04,"z":-0.4,"w":0.75,"d":0.75,"h":0.03,"tint":"#2a2622"}'),
  ('legs','box','mat.cast_iron',0,'','{"x":-1.05,"z":-0.4,"w":0.12,"d":0.14,"h":3.1,"roll":6}'),
  ('legs_e','box','mat.cast_iron',0,'','{"x":-0.15,"z":-0.4,"w":0.12,"d":0.14,"h":3.1,"roll":-6}'),
  ('stays','box','mat.cast_iron',0,'','{"x":-0.75,"z":-1.55,"w":0.1,"d":0.1,"h":3.2,"pitch":20,"roll":4}'),
  ('stays_e','box','mat.cast_iron',0,'','{"x":-0.45,"z":-1.55,"w":0.1,"d":0.1,"h":3.2,"pitch":20,"roll":-4,"far":0}'),
  ('braces','box','mat.cast_iron',0,'','{"x":-0.6,"y":1,"z":-0.4,"w":0.9,"d":0.06,"h":0.06,"repeat":1}'),
  ('braces_2','box','mat.cast_iron',0,'','{"x":-0.6,"y":2,"z":-0.4,"w":0.82,"d":0.06,"h":0.06,"far":0}'),
  ('head','box','mat.cast_iron',0,'','{"x":-0.6,"y":3.05,"z":-0.45,"w":0.95,"d":0.5,"h":0.12}'),
  ('sheave','shield','mat.steel',0,'','{"x":-0.6,"y":3.35,"z":-0.3,"yaw":90,"w":0.7,"t":0.05,"boss":0.06,"segments":12,"rim":"#4a4e56"}'),
  ('sheave_2','shield','mat.steel',0,'','{"x":-0.6,"y":3.35,"z":-0.6,"yaw":90,"w":0.7,"t":0.05,"boss":0.06,"segments":12,"rim":"#4a4e56","far":0}'),
  ('head_cap','gable_roof','mat.cloth',1,'','{"x":-0.6,"y":3.17,"z":-0.45,"w":1,"d":0.5,"pitch":30,"overhang":0.04,"t":0.04}'),
  ('ropes','box','mat.steel',0,'','{"x":-0.6,"y":0.9,"z":0.75,"w":0.03,"d":0.03,"h":2.73,"pitch":-26,"far":0}'),
  ('cage','box','mat.cast_iron',0,'','{"x":-0.6,"y":0.4,"z":-0.4,"w":0.4,"d":0.4,"h":0.5}'),
  ('winding_house','box','mat.red_brick',0,'','{"x":-0.6,"z":1.2,"w":1.3,"d":0.9,"h":1.1,"door":0.3,"door_h":0.55,"windows":2,"window_y":0.4,"window_w":0.16,"window_h":0.36,"opening":"#22282f"}'),
  ('winding_roof','gable_roof','mat.slate',0,'','{"x":-0.6,"y":1.1,"z":1.2,"w":1.3,"d":0.9,"pitch":32,"overhang":0.08,"t":0.06}'),
  ('winding_roof_cap','gable_roof','mat.cloth',1,'','{"x":-0.6,"y":1.328,"z":1.2,"w":1.46,"d":0.396,"pitch":32,"overhang":0,"t":0.03}'),
  ('chimney','cylinder','mat.red_brick',0,'','{"x":0.25,"y":0,"z":1.35,"w":0.36,"h":2.6,"taper":0.3,"segments":8}'),
  ('chimney_band','cylinder','mat.cloth',1,'','{"x":0.25,"y":1.976,"z":1.35,"w":0.302,"h":0.286,"segments":8}'),
  ('chimney_cap','cylinder','mat.cast_iron',0,'','{"x":0.25,"y":2.6,"z":1.35,"w":0.288,"h":0.12,"segments":8,"far":0}'),
  ('spoil','cone','mat.spoil',0,'','{"x":1,"z":-0.3,"w":1.6,"h":1,"segments":8}'),
  ('spoil_back','cone','mat.spoil',0,'','{"x":1.25,"z":-1.35,"w":1.1,"h":0.6,"segments":7}'),
  ('tubs_rails_bed','box','mat.cinders',0,'','{"x":0.35,"y":0,"z":0.4,"w":1.6,"d":0.5,"h":0.05}'),
  ('tubs_rails_sleepers','box','mat.oak',0,'tubs_rails_bed','{"x":-0.65,"y":0.05,"w":0.08,"d":0.44,"h":0.02,"repeat":2,"dx":0.6,"far":0}'),
  ('tubs_rails','box','mat.cast_iron',0,'tubs_rails_bed','{"y":0.07,"z":-0.11,"w":1.6,"d":0.04,"h":0.04,"repeat":2,"dz":0.22,"far":0}'),
  ('tub','box','mat.cast_iron',0,'','{"x":0.65,"y":0.12,"z":0.4,"w":0.4,"d":0.32,"h":0.26}'),
  ('ore','dome','mat.ore',0,'','{"x":0.65,"y":0.38,"z":0.4,"w":0.36,"h":0.14,"segments":6,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.5,"z":1.5,"h":2.4,"fw":0.5,"fh":0.75}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.mine_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.mine'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.mine_industrial'), (SELECT id FROM def WHERE key='mat.cast_iron'), 100);
