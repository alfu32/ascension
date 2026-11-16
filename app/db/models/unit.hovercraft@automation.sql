-- unit.hovercraft@automation: Hovercraft (mesh.hovercraft_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.hovercraft_automation','Hovercraft');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.hovercraft_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('skirt','box','mat.rubber',0,'','{"y":0.06,"w":1.2,"d":1.6,"h":0.26,"taper":-0.06,"bank":5}'),
  ('skirt_bow','cylinder','mat.rubber',0,'skirt','{"z":0.8,"w":1.2,"h":0.26,"segments":10}'),
  ('hull','box','mat.white_composite',0,'skirt','{"y":0.26,"z":0.05,"w":1.12,"d":1.7,"h":0.1}'),
  ('hull_bow','cylinder','mat.white_composite',0,'skirt','{"y":0.26,"z":0.8,"w":1.12,"h":0.1,"segments":10,"far":0}'),
  ('band','box','mat.white_composite',1,'skirt','{"y":0.26,"z":-0.1,"w":1.15,"d":1.4,"h":0.06}'),
  ('cabin','box','mat.white_composite',0,'skirt','{"y":0.36,"z":0.35,"w":0.72,"d":0.82,"h":0.3,"taper":0.25}'),
  ('glass','box','mat.smart_glass',0,'skirt','{"y":0.47,"z":0.35,"w":0.66,"d":0.76,"h":0.1,"taper":0.08,"far":0}'),
  ('cabin_roof','box','mat.white_composite',1,'skirt','{"y":0.66,"z":0.35,"w":0.54,"d":0.62,"h":0.03}'),
  ('ducts','torus','mat.white_composite',1,'skirt','{"x":-0.3,"y":0.66,"z":-0.68,"w":0.56,"t":0.08,"segments":10,"sides":4,"roll":90,"yaw":90,"repeat":2,"dx":0.6,"far":0}'),
  ('fan_w','propeller','mat.composite',0,'skirt','{"x":-0.3,"y":0.66,"z":-0.7,"w":0.48,"blades":3,"chord":0.1}'),
  ('fan_e','propeller','mat.composite',0,'skirt','{"x":0.3,"y":0.66,"z":-0.7,"w":0.48,"blades":3,"chord":0.1}'),
  ('rudders','box','mat.white_composite',1,'skirt','{"x":-0.3,"y":0.4,"z":-0.98,"w":0.03,"d":0.14,"h":0.5,"repeat":2,"dx":0.6}'),
  ('strips','box','mat.light_strip',0,'skirt','{"x":-0.565,"y":0.29,"z":0.05,"w":0.02,"d":1.4,"h":0.03,"repeat":2,"dx":1.13,"far":0}'),
  ('mount','box','mat.graphite',0,'skirt','{"y":0.69,"z":0.25,"w":0.2,"d":0.24,"h":0.1,"traverse":30,"far":0}'),
  ('gun','cylinder','mat.graphite',0,'mount','{"y":0.05,"z":0.1,"w":0.04,"h":0.42,"segments":4,"pitch":90,"far":0}'),
  ('spark','cone','mat.flash',0,'gun','{"y":0.42,"w":0.08,"h":0.08,"segments":3,"flash":4,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.26,"w":0.26,"h":0.26,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.hovercraft_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.hovercraft'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.hovercraft_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
