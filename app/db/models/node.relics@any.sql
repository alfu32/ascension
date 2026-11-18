-- node.relics@any: Relic site (mesh.relic_site)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.relic_site','Relic site');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.relic_site';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('platform','plinth','mat.fieldstone',0,'','{"w":1.8,"d":1.8,"h":0.1,"steps":1}'),
  ('terrace','box','mat.fieldstone',0,'','{"y":0.1,"z":-0.23,"w":1.56,"d":1.1,"h":0.1}'),
  ('terrace_w','box','mat.fieldstone',0,'','{"x":-0.33,"y":0.1,"z":0.55,"w":0.9,"d":0.46,"h":0.1}'),
  ('dais','plinth','mat.dressed_stone',0,'','{"y":0.1,"z":-0.05,"w":1.1,"d":1.0,"h":0.22,"steps":2,"inset":0.05,"tint":"#e0dccb"}'),
  ('fallen_step','box','mat.fieldstone',0,'','{"x":0.34,"y":0.1,"z":0.79,"yaw":8,"roll":9,"w":0.3,"d":0.14,"h":0.1}'),
  ('loose_block','box','mat.fieldstone',0,'','{"x":-0.8,"z":-0.42,"yaw":-22,"w":0.18,"d":0.22,"h":0.12,"far":0}'),
  ('moss','box','mat.turf',0,'','{"x":-0.55,"y":0.2,"z":0.55,"yaw":18,"w":0.32,"d":0.24,"h":0.015,"far":0}'),
  ('moss_corner','box','mat.turf',0,'','{"x":0.4,"y":0.1,"z":0.4,"yaw":-10,"w":0.3,"d":0.22,"h":0.015,"far":0}'),
  ('moss_back','box','mat.turf',0,'','{"x":0.62,"y":0.2,"z":-0.58,"yaw":-30,"w":0.26,"d":0.2,"h":0.015,"far":0}'),
  ('upright_w','box','mat.rough_stone',0,'','{"x":-0.48,"y":0.2,"z":-0.72,"roll":-2,"w":0.24,"d":0.2,"h":1.0,"taper":0.12}'),
  ('upright_e','box','mat.rough_stone',0,'','{"x":0.48,"y":0.2,"z":-0.72,"roll":2,"w":0.24,"d":0.2,"h":1.0,"taper":0.12}'),
  ('lintel','box','mat.rough_stone',0,'','{"y":1.18,"z":-0.72,"roll":3,"w":1.34,"d":0.24,"h":0.18}'),
  ('stone_w','box','mat.rough_stone',0,'','{"x":-0.8,"y":0.1,"z":0.1,"roll":-7,"pitch":4,"w":0.18,"d":0.2,"h":0.82,"taper":0.25}'),
  ('stone_e','box','mat.rough_stone',0,'','{"x":0.8,"y":0.1,"z":-0.05,"roll":5,"w":0.2,"d":0.18,"h":0.6,"taper":0.2}'),
  ('stump','cylinder','mat.limestone',0,'','{"x":-0.62,"y":0.2,"z":0.62,"w":0.24,"h":0.42,"segments":8,"tint":"#dcd6c4"}'),
  ('capital','box','mat.limestone',0,'stump','{"y":0.42,"yaw":12,"w":0.3,"d":0.3,"h":0.08,"tint":"#dcd6c4","far":0}'),
  ('fallen_column','cylinder','mat.limestone',0,'','{"x":0.14,"y":0.2,"z":0.5,"yaw":-15,"roll":90,"w":0.2,"h":0.64,"segments":8,"bottom":1,"tint":"#dcd6c4"}'),
  ('drum','cylinder','mat.limestone',0,'','{"x":0.68,"y":0.1,"z":0.4,"yaw":20,"w":0.2,"h":0.14,"segments":8,"tint":"#d2ccb8","far":0}'),
  ('altar','box','mat.limestone',0,'','{"y":0.32,"z":-0.12,"w":0.6,"d":0.42,"h":0.26,"tint":"#e6e0cf"}'),
  ('altar_cloth','box','mat.cloth',0,'','{"y":0.58,"z":-0.12,"w":0.64,"d":0.46,"h":0.02,"tint":"#7a2a44"}'),
  ('drape','box','mat.cloth',0,'','{"y":0.4,"z":0.105,"w":0.42,"d":0.02,"h":0.2,"tint":"#7a2a44"}'),
  ('reliquary','box','mat.gold',0,'','{"y":0.6,"z":-0.12,"w":0.48,"d":0.28,"h":0.2}'),
  ('reliquary_roof','gable_roof','mat.gold',0,'reliquary','{"y":0.2,"w":0.48,"d":0.28,"pitch":40,"overhang":0.03,"t":0.03}'),
  ('reliquary_band','box','mat.lime_plaster',0,'reliquary','{"y":0.07,"w":0.49,"d":0.29,"h":0.05,"tint":"#fff6dc","far":0}'),
  ('finials','dome','mat.gold',0,'reliquary','{"x":-0.24,"y":0.31,"w":0.07,"h":0.07,"segments":6,"repeat":2,"dx":0.48,"far":0}'),
  ('urn_w','cylinder','mat.gold',0,'','{"x":-0.42,"y":0.32,"z":0.14,"w":0.14,"h":0.2,"taper":0.35,"segments":8}'),
  ('urn_w_lid','dome','mat.gold',0,'urn_w','{"y":0.2,"w":0.1,"h":0.07,"segments":8,"far":0}'),
  ('urn_e','cylinder','mat.gold',0,'','{"x":0.4,"y":0.32,"z":0.12,"w":0.12,"h":0.16,"taper":0.35,"segments":8}'),
  ('bowl','dome','mat.gold',0,'','{"x":0.2,"y":0.38,"z":0.26,"pitch":180,"w":0.14,"h":0.06,"segments":8,"far":0}'),
  ('candles','cylinder','mat.lime_plaster',0,'','{"x":-0.44,"y":0.32,"z":-0.4,"w":0.04,"h":0.14,"segments":4,"repeat":2,"dx":0.88,"far":0}'),
  ('flames','cone','mat.glow',0,'','{"x":-0.44,"y":0.46,"z":-0.4,"w":0.035,"h":0.06,"segments":4,"repeat":2,"dx":0.88,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.relic_site'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='node.relics'), NULL, NULL, (SELECT id FROM def WHERE key='mesh.relic_site'), (SELECT id FROM def WHERE key='mat.fieldstone'), 100);
