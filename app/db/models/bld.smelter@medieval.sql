-- bld.smelter@medieval: Medieval smelter (mesh.smelter_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.smelter_medieval','Medieval smelter');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.smelter_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1,"tint":"#a89a88"}'),
  ('furnace','box','mat.rough_stone',0,'','{"x":-0.35,"z":-0.55,"w":1.7,"d":1.6,"h":1.3,"taper":0.18}'),
  ('furnace_top','box','mat.rough_stone',0,'','{"x":-0.35,"y":1.3,"z":-0.55,"w":1.5,"d":1.4,"h":0.18,"tint":"#cfc8bc"}'),
  ('mouth','arch','mat.dressed_stone',0,'','{"x":-0.35,"z":0.25,"w":0.9,"h":0.85,"t":0.14,"span":0.5,"spring":0.3}'),
  ('glow','box','mat.glow',0,'','{"x":-0.35,"y":0.03,"z":0.2,"w":0.5,"d":0.04,"h":0.52}'),
  ('chimney','box','mat.dressed_stone',0,'','{"x":-0.35,"y":1.48,"z":-0.55,"w":0.75,"d":0.75,"h":2.35,"taper":0.22}'),
  ('chimney_lip','box','mat.dressed_stone',0,'','{"x":-0.35,"y":3.8,"z":-0.55,"w":0.66,"d":0.66,"h":0.14,"tint":"#cfc8bc"}'),
  ('chimney_glow','box','mat.glow',0,'','{"x":-0.35,"y":3.86,"z":-0.55,"w":0.46,"d":0.46,"h":0.1}'),
  ('shed_posts','box','mat.oak',0,'','{"x":0.85,"z":-1.55,"w":0.12,"d":0.12,"h":1.35,"repeat":2,"dx":0.9}'),
  ('shed_posts_front','box','mat.oak',0,'','{"x":0.85,"z":-0.6,"w":0.12,"d":0.12,"h":1.15,"repeat":2,"dx":0.9,"far":0}'),
  ('shed_roof','awning','mat.shingle',0,'','{"x":1.3,"y":0,"z":-1.08,"w":1.25,"d":1.1,"h":1.15,"rise":0.3,"posts":0}'),
  ('shed_trim','box','mat.cloth',1,'','{"x":1.3,"y":1.04,"z":-0.52,"w":1.25,"d":0.04,"h":0.16}'),
  ('bellows','box','mat.leather',0,'','{"x":1.05,"y":0.3,"z":-1.1,"w":0.7,"d":0.5,"h":0.32,"taper":0.4}'),
  ('bellows_board','box','mat.oak',0,'','{"x":1.05,"y":0.62,"z":-1.1,"w":0.75,"d":0.55,"h":0.05,"roll":8,"far":0}'),
  ('pipe','box','mat.iron',0,'','{"x":0.5,"y":0.45,"z":-1.1,"w":0.4,"d":0.08,"h":0.08,"far":0}'),
  ('ore','cone','mat.ore',0,'','{"x":1.15,"z":0.75,"w":1.3,"h":0.7,"segments":8}'),
  ('charcoal','dome','mat.charcoal',0,'','{"x":-1.2,"z":1,"w":1,"h":0.42,"segments":8}'),
  ('ingots','box','mat.iron',0,'','{"x":0.25,"z":1.15,"w":0.5,"d":0.32,"h":0.12,"repeat":2,"dx":0.55,"tint":"#a0a6ae"}'),
  ('awning','awning','mat.cloth',1,'','{"x":0.5,"z":1.15,"w":1.3,"d":0.7,"h":0.95,"rise":0.25}'),
  ('banner','banner','mat.cloth',1,'','{"x":-1.5,"z":-1.4,"h":2.6,"fw":0.5,"fh":0.8}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.smelter_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.smelter'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.smelter_medieval'), (SELECT id FROM def WHERE key='mat.rough_stone'), 100);
